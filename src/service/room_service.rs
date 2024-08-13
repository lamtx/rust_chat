use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use hyper::{Request, StatusCode};
use hyper_tls::HttpsConnector;
use hyper_tungstenite::tungstenite::error::ProtocolError;
use hyper_tungstenite::tungstenite::Message as WsMessage;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::Serialize;
use tokio::sync::mpsc::channel;

use crate::config::PORT;
use crate::misc::*;
use crate::model::{JoinParams, Message, Participant, RoomInfo, TextRoomEvent};
use crate::service::client_service::ChatClient;
use crate::service::Room;
use crate::{command, log};

command! {
    pub Status() -> RoomInfo;
    pub Count() -> usize;
    pub AddClient(client: ChatClient, image_url: Option<String>);
    pub RemoveClient(id: usize);
    pub Announce(sender: Participant, r#type: String, text: String);
    pub Ban(from: Option<String>, victim: String);
    pub Broadcast(message: String);
    pub GetNextId()  -> usize;
    pub LastAnnouncement(types: Vec<String>) -> HashMap<String, String>;
    pub Participants() -> Vec<Participant>;
    /// Return the serialized json of all [Message] in this room.
    pub AllMessages() -> String;
    pub Photo(username: String) -> Option<String>;
    pub SendMessage(sender: Participant, r#type: String, text: String);
    pub Destroy();
}

#[derive(Clone)]
pub struct ChatRoom {
    pub secret: Arc<String>,
    pub op: CommandSender,
}

impl ChatRoom {
    pub fn create<F>(room: Room, on_room_detached: F) -> ChatRoom
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let (tx, mut rx) = channel::<Command>(30);
        let chat_room = ChatRoom {
            op: CommandSender { tx },
            secret: Arc::new(room.secret.clone()),
        };
        tokio::spawn(async move {
            let mut state = ChatRoomInner::new(room, on_room_detached);
            log!("`room {}` created", &state.room.uid);
            log!(
                "Click here to destroy: http://127.0.0.1:{}{}/destroy?secret={}",
                PORT, &state.room.uid, &state.room.secret
            );
            state.post_created();
            while let Some(command) = rx.recv().await {
                match command {
                    Command::Status { resp_tx } => {
                        let _ = resp_tx.send(state.status());
                    }
                    Command::Count { resp_tx } => {
                        let _ = resp_tx.send(state.count());
                    }
                    Command::AddClient {
                        client,
                        image_url,
                        resp_tx,
                    } => {
                        state.add_client(client, image_url);
                        let _ = resp_tx.send(());
                    }
                    Command::Announce {
                        sender,
                        r#type,
                        text,
                        resp_tx,
                    } => {
                        state.announce(sender, r#type, text);
                        let _ = resp_tx.send(());
                    }
                    Command::SendMessage {
                        sender,
                        r#type,
                        text,
                        resp_tx,
                    } => {
                        state.send_message(sender, r#type, text);
                        let _ = resp_tx.send(());
                    }
                    Command::Ban {
                        from,
                        victim,
                        resp_tx,
                    } => {
                        state.ban(from, victim);
                        let _ = resp_tx.send(());
                    }
                    Command::Broadcast { message, resp_tx } => {
                        state.broadcast(message);
                        let _ = resp_tx.send(());
                    }
                    Command::GetNextId { resp_tx } => {
                        state.next_id += 1;
                        let _ = resp_tx.send(state.next_id);
                    }
                    Command::RemoveClient { id, resp_tx } => {
                        if !state.clients.is_empty() {
                            state.clients.retain(|e| e.id != id);
                            log!("client id:{id} detached (size: {})", state.clients.len());
                        }
                        let _ = resp_tx.send(());
                    }
                    Command::Destroy { resp_tx } => {
                        state.destroy();
                        let _ = resp_tx.send(());
                    }
                    Command::LastAnnouncement { types, resp_tx } => {
                        let _ = resp_tx.send(state.last_announcement(types));
                    }
                    Command::Participants { resp_tx } => {
                        let _ = resp_tx.send(state.participants());
                    }
                    Command::AllMessages { resp_tx } => {
                        let json = serde_json::to_string(&state.messages).unwrap();
                        let _ = resp_tx.send(json);
                    }
                    Command::Photo { username, resp_tx } => {
                        let photo = state.photos.get(&username).map(|e| e.to_owned());
                        let _ = resp_tx.send(photo);
                    }
                }
            }
            log!("room `{}` dropped", &state.room.uid);
        });

        chat_room
    }

    pub async fn join(
        &self,
        mut req: HttpRequest,
        params: JoinParams,
    ) -> Result<HttpResponse, ProtocolError> {
        use hyper_tungstenite::*;
        if is_upgrade_request(&req) {
            let (response, websocket) = upgrade(&mut req, None)?;
            let this = self.clone();
            let id = this.op.GetNextId().await;
            let me = Participant {
                username: params.username,
                display: params.display,
            };
            tokio::spawn(async move {
                let client = ChatClient::create(websocket, this.clone(), me, id).await;
                match client {
                    Ok(client) => {
                        this.op.AddClient(client, params.image_url).await;
                    }
                    Err(e) => {
                        eprintln!("create client failed: {:?}", e);
                    }
                }
            });
            Ok(response)
        } else {
            log!("The request is not upgradable to web socket");
            Err(ProtocolError::MissingConnectionUpgradeHeader)
            // Or MissingUpgradeWebSocketHeader but it's not importance
        }
    }
}

struct ChatRoomInner<F>
where
    F: Fn(String),
{
    room: Room,
    messages: Vec<Message>,
    clients: Vec<ChatClient>,
    photos: HashMap<String, String>,
    next_id: usize,
    detached: bool,
    on_room_detached: F,
}

impl<F> ChatRoomInner<F>
where
    F: Fn(String),
{
    fn new(room: Room, on_room_detached: F) -> Self {
        ChatRoomInner {
            room,
            clients: Vec::new(),
            messages: Vec::new(),
            photos: HashMap::new(),
            next_id: 0,
            detached: false,
            on_room_detached,
        }
    }

    fn status(&self) -> RoomInfo {
        RoomInfo {
            room: self.room.uid.clone(),
            participants: self.clients.iter().map(|e| e.me.clone()).collect(),
            messages: self.messages.len(),
        }
    }

    fn count(&self) -> usize {
        self.clients.len()
    }

    fn participants(&self) -> Vec<Participant> {
        self.clients.iter().map(|e| e.me.clone()).collect()
    }

    fn last_announcement(&self, types: Vec<String>) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for r#type in types {
            let message = self
                .messages
                .iter()
                .rfind(|x| x.textroom == Message::ANNOUNCEMENT && x.r#type == r#type);
            if let Some(text) = message {
                result.insert(r#type, text.r#type.clone());
            }
        }

        result
    }

    fn add_client(&mut self, client: ChatClient, image_url: Option<String>) {
        if client.me.username.is_some() && image_url.is_some() {
            self.photos
                .insert(client.me.username.clone().unwrap(), image_url.unwrap());
        }
        let participant = &client.me;
        let event = &TextRoomEvent::Joined {
            username: participant.username.as_deref(),
            display: participant.display.as_deref(),
            participants: self.count(),
        };
        self.broadcast_json(event);
        log!(
            "'{}' joined (id: {}, count:{})",
            participant.display.or_empty(),
            client.id,
            self.clients.len() + 1
        );

        self.clients.push(client);
    }
    fn send_message(&mut self, sender: Participant, r#type: String, text: String) {
        if sender.username.is_none() || sender.display.is_none() {
            return;
        }
        let now = Utc::now();
        let username = sender.username.unwrap();
        let display = sender.display.unwrap();

        self.broadcast_json(&TextRoomEvent::Message {
            from: &username,
            display: &display,
            date: now,
            text: &text,
            r#type: &r#type,
        });

        let message = Message {
            room: self.room.name().to_string(),
            textroom: Message::MESSAGE,
            r#type,
            text,
            date: now,
            from: username,
        };

        self.post(&message, true);
        self.messages.push(message);
    }
    fn announce(&mut self, sender: Participant, r#type: String, text: String) {
        let now = Utc::now();
        self.broadcast_json(&TextRoomEvent::Announcement {
            date: now,
            text: &text,
            r#type: &r#type,
        });

        let message = Message {
            room: self.room.name().to_string(),
            textroom: Message::ANNOUNCEMENT,
            r#type,
            text,
            date: now,
            from: sender.username.unwrap(),
        };
        self.post(&message, true);
        self.messages.push(message);
    }

    fn ban(&self, from: Option<String>, victim: String) {
        log!("{:?} wants to ban {victim}", from);
        let victims = self
            .clients
            .iter()
            .filter(|&e| e.me.username.contains(&victim));
        let event = serde_json::to_string(&TextRoomEvent::Banned).unwrap();
        for client in victims {
            client.op.spawn().Send(WsMessage::Text(event.clone()));
            client.op.spawn().Leave();
        }
        self.post(
            &Message {
                textroom: Message::MODERATE,
                room: self.room.name().to_string(),
                r#type: Message::TYPE_BAN.into(),
                text: victim,
                date: Utc::now(),
                from: from.unwrap_or_default(),
            },
            false,
        );
    }

    fn post_created(&self) {
        self.post(&Message::room_created(self.room.name().to_string()), false);
    }

    fn post(&self, message: &Message, should_check_type: bool) {
        if self.room.post.is_none() {
            return;
        }
        if should_check_type && !self.room.post_types.contains(&message.r#type) {
            return;
        }
        let request = Request::builder()
            .uri(self.room.post.as_ref().unwrap())
            .method("POST")
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(message).unwrap())
            .unwrap();
        let https = HttpsConnector::new();
        let client = Client::builder(TokioExecutor::new()).build(https);
        tokio::spawn(async move {
            match client.request(request).await {
                Ok(response) => {
                    if response.status() != StatusCode::OK {
                        eprintln!("posted failed, status code:{}", response.status());
                    }
                }
                Err(e) => {
                    eprintln!("posted error: {:?}", e);
                }
            }
        });
    }

    fn broadcast_json<T: Serialize>(&self, body: &T) {
        let content = serde_json::to_string(body).unwrap();
        self.broadcast(content)
    }

    fn broadcast(&self, body: String) {
        for client in &self.clients {
            client.op.spawn().Send(WsMessage::Text(body.clone()))
        }
    }

    fn detach(&mut self) {
        if !self.detached {
            self.detached = true;
            self.broadcast_json(&TextRoomEvent::Destroyed);
            self.clients.clear();
            (self.on_room_detached)(self.room.uid.clone());
            log!("room `{}` destroyed", self.room.name())
        }
    }
    fn destroy(&mut self) {
        self.detach();
        self.post(
            &Message::room_destroyed(self.room.name().to_string()),
            false,
        )
    }
}
