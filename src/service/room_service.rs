use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use chrono::Utc;
use hyper_tungstenite::tungstenite::error::ProtocolError;
use hyper_tungstenite::tungstenite::Message as WsMessage;
use serde::Serialize;

use crate::config::PORT;
use crate::misc::*;
use crate::model::{JoinParams, Message, Participant, Room, RoomInfo, TextRoomEvent};
use crate::service::client_service::ChatClient;
use crate::service::rest_client::RestClient;
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
        let (op, mut rx) = Command::new_channel();
        let chat_room = ChatRoom {
            op,
            secret: Arc::new(room.secret.clone()),
        };
        tokio::spawn(async move {
            let mut state = ChatRoomInner::new(room, on_room_detached);
            log!("`room {}` created", &state.room.uid);
            log!(
                "To destroy: http://127.0.0.1:{}{}/destroy?secret={}",
                PORT,
                &state.room.uid,
                &state.room.secret
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
                        log!("create client failed: {:?}", e);
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
    clients: Vec<ChatClient>,
    photos: HashMap<String, String>,
    last_announcements: HashMap<String, String>,
    rest_client: Option<RestClient>,
    // cache value from [self.room.name()]
    room_name: String,
    messages: usize,
    next_id: usize,
    detached: bool,
    on_room_detached: F,
}

impl<F> ChatRoomInner<F>
where
    F: Fn(String),
{
    fn new(room: Room, on_room_detached: F) -> Self {
        let rest_client = match &room.post {
            None => None,
            Some(post) => Some(RestClient::create(post.clone())),
        };
        ChatRoomInner {
            room_name: room.name().to_string(),
            room,
            clients: Vec::new(),
            last_announcements: HashMap::new(),
            photos: HashMap::new(),
            next_id: 0,
            messages: 0,
            detached: false,
            on_room_detached,
            rest_client,
        }
    }

    fn status(&self) -> RoomInfo {
        RoomInfo {
            room: self.room.uid.clone(),
            participants: self.clients.iter().map(|e| e.me.clone()).collect(),
            messages: self.messages,
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
            if let Some(announcement) = self.last_announcements.get(&r#type) {
                result.insert(r#type, announcement.clone());
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
        self.messages += 1;

        self.post(
            &Message {
                room: &self.room_name,
                textroom: Message::MESSAGE,
                r#type: &r#type,
                text: &text,
                date: now,
                from: &username,
            },
            true,
        );
    }
    fn announce(&mut self, sender: Participant, r#type: String, text: String) {
        let now = Utc::now();
        self.broadcast_json(&TextRoomEvent::Announcement {
            date: now,
            text: &text,
            r#type: &r#type,
        });
        self.messages += 1;

        self.post(
            &Message {
                room: &self.room_name,
                textroom: Message::ANNOUNCEMENT,
                r#type: &r#type,
                text: &text,
                date: now,
                from: &sender.username.unwrap(),
            },
            true,
        );

        self.last_announcements.insert(r#type, text);
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
                room: &self.room_name,
                r#type: Message::TYPE_BAN.into(),
                text: &victim,
                date: Utc::now(),
                from: from.or_empty(),
            },
            false,
        );
    }

    fn post_created(&self) {
        self.post(&Message::room_created(&self.room_name), false);
    }

    fn post(&self, message: &Message, should_check_type: bool) {
        if let Some(rest_client) = &self.rest_client {
            if should_check_type
                && !self
                    .room
                    .post_types
                    .iter()
                    .any(|e| e.deref() == message.r#type)
            {
                return;
            }
            rest_client.spawn_post(message);
        }
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
            log!("room `{}` destroyed", self.room_name)
        }
    }
    fn destroy(&mut self) {
        self.detach();
        self.post(&Message::room_destroyed(&self.room_name), false)
    }
}
