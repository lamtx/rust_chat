use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use futures::{StreamExt, TryStreamExt};
use hyper_tungstenite::tungstenite::error::ProtocolError;
use hyper_tungstenite::tungstenite::Message as WsMessage;
use serde::{Deserialize, Serialize};

use crate::{command, log};
use crate::config::PORT;
use crate::misc::*;
use crate::model::{
    JoinParams, Message, Participant, Room, RoomInfo, TextRoomEvent, TextRoomRequest,
    TextRoomResponse,
};
use crate::service::client_service::ChatClient;
use crate::service::rest_client::RestClient;

command! {
    pub Status() -> RoomInfo;
    pub Count() -> usize;
    pub Join(sink: WebSocketSink, params: JoinParams) -> usize;
    pub LastAnnouncement(types: Vec<String>) -> HashMap<String, String>;
    pub Participants() -> Vec<Participant>;
    pub Photo(username: String) -> Option<String>;
    pub Destroy();
    OnMessageReceived(sender_id:usize, message: WsMessage);
    Leave(id: usize);
}

#[derive(Clone)]
pub struct ChatRoom {
    pub secret: Arc<String>,
    pub op: CommandSender,
}

impl ChatRoom {
    pub fn create(room: Room) -> ChatRoom {
        let (op, mut rx) = Command::new_channel();
        let chat_room = ChatRoom {
            op,
            secret: Arc::new(room.secret.clone()),
        };
        tokio::spawn(async move {
            let mut state = ChatRoomInner::new(room);
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
                    Command::Join {
                        sink,
                        params,
                        resp_tx,
                    } => {
                        let _ = resp_tx.send(state.join(sink, params));
                    }
                    Command::Leave { id, resp_tx } => {
                        state.leave(id);
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
                    Command::OnMessageReceived {
                        sender_id,
                        message,
                        resp_tx,
                    } => {
                        let _ = resp_tx.send(state.on_message_received(sender_id, message).await);
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
            let (response, socket) = upgrade(&mut req, None)?;
            let this = self.clone();

            tokio::spawn(async move {
                // FIXME: do not unwrap
                let (sink, mut stream) = socket.await.unwrap().split();
                let id = this.op.Join(sink, params).await;
                while let Some(message) = stream.try_next().await.ok().flatten() {
                    this.op.OnMessageReceived(id, message).await;
                }
                this.op.Leave(id).await;
            });
            Ok(response)
        } else {
            log!("The request is not upgradable to web socket");
            Err(ProtocolError::MissingConnectionUpgradeHeader)
            // Or MissingUpgradeWebSocketHeader but it's not importance
        }
    }
}

struct ChatRoomInner {
    room: Room,
    clients: HashMap<usize, ChatClient>,
    photos: HashMap<String, String>,
    last_announcements: HashMap<String, String>,
    rest_client: Option<RestClient>,
    // cache value from [self.room.name()]
    room_name: String,
    messages: usize,
    next_id: usize,
    is_destroyed: bool,
}

impl ChatRoomInner {
    fn new(room: Room) -> Self {
        let rest_client = match &room.post {
            None => None,
            Some(post) => Some(RestClient::create(post.clone())),
        };
        ChatRoomInner {
            room_name: room.name().to_string(),
            room,
            clients: HashMap::new(),
            last_announcements: HashMap::new(),
            photos: HashMap::new(),
            next_id: 0,
            messages: 0,
            is_destroyed: false,
            rest_client,
        }
    }
    fn join(&mut self, socket: WebSocketSink, params: JoinParams) -> usize {
        self.next_id += 1;
        let id = self.next_id;
        let me = Participant {
            username: params.username,
            display: params.display,
        };
        let client = ChatClient::new(socket, me);
        if client.me.username.is_some() && params.image_url.is_some() {
            self.photos.insert(
                client.me.username.clone().unwrap(),
                params.image_url.unwrap(),
            );
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
            id,
            self.clients.len() + 1
        );

        self.clients.insert(id, client);
        id
    }

    fn leave(&mut self, id: usize) -> Option<ChatClient> {
        if let Some(client) = self.clients.remove(&id) {
            let len = self.clients.len();
            println!("`{}` left (size={len})", client.me.display.or_empty(),);
            let event = &TextRoomEvent::Left {
                username: client.me.username.as_deref(),
                display: client.me.display.as_deref(),
                participants: len,
            };
            self.broadcast(serde_json::to_string(event).unwrap());
            Some(client)
        } else {
            None
        }
    }

    async fn on_message_received(&mut self, sender_id: usize, message: WsMessage) {
        match message {
            WsMessage::Text(text) => {
                self.on_listen(sender_id, text);
            }
            WsMessage::Binary(msg) => {
                log!("unexpected binary message: {:02X?}", msg);
            }
            WsMessage::Ping(_) => {}
            WsMessage::Pong(_) => {
                let now = Instant::now();
                log!("received pong at {:?}", now);
                // self.last_pong = now;
            }
            WsMessage::Close(msg) => {
                if let Some(msg) = &msg {
                    log!(
                        "received close message with code {} and message: {}",
                        msg.code,
                        msg.reason
                    );
                } else {
                    log!("Received close message");
                }
                // TODO: should close this instance?
            }
            WsMessage::Frame(_) => {}
        }
    }
    fn on_listen(&mut self, sender_id: usize, message: String) {
        log!("receive: {message}");

        let response = match serde_json::from_str(&message) {
            Ok(value) => self.handle_request(sender_id, value),
            Err(e) => {
                eprintln!("parse json failed: {:?}", e);
                serde_json::from_str::<UnknownTextRoomRequest>(&message)
                    .map(|e| e.transaction)
                    .unwrap_or(None)
                    .map(|transaction| TextRoomResponse::Error {
                        transaction: Some(transaction),
                        error: e.to_string(),
                    })
            }
        };
        if let Some(response) = response {
            log!("reply: {:?}", response);
            self.reply_json(sender_id, &response);
        }
    }

    fn handle_request(
        &mut self,
        sender_id: usize,
        request: TextRoomRequest,
    ) -> Option<TextRoomResponse> {
        if self.is_destroyed {
            return Some(TextRoomResponse::destroyed(request.transaction()));
        }
        log!("handling ws message: {:?}", &request);
        match request {
            TextRoomRequest::Message { r#type, text, .. } => {
                self.send_message(sender_id, r#type, text);
                None
            }
            TextRoomRequest::Announcement {
                secret,
                r#type,
                text,
                transaction,
                ..
            } => {
                if self.room.secret.deref() == &secret {
                    self.announce(sender_id, r#type, text);
                    None
                } else {
                    Some(TextRoomResponse::secret(transaction))
                }
            }
            TextRoomRequest::Leave { transaction } => {
                self.leave(sender_id);
                Some(TextRoomResponse::left(transaction))
            }
            TextRoomRequest::Ban {
                secret,
                username,
                transaction,
            } => {
                if self.room.secret.deref() == &secret {
                    self.ban(sender_id, username);
                    None
                } else {
                    Some(TextRoomResponse::secret(transaction))
                }
            }
        }
    }

    fn status(&self) -> RoomInfo {
        RoomInfo {
            room: self.room.uid.clone(),
            participants: self.clients.iter().map(|(_, e)| e.me.clone()).collect(),
            messages: self.messages,
        }
    }

    fn count(&self) -> usize {
        self.clients.len()
    }

    fn participants(&self) -> Vec<Participant> {
        self.clients.iter().map(|(_, e)| e.me.clone()).collect()
    }

    fn participant_by_id(&self, id: usize) -> Option<&Participant> {
        self.clients.get(&id).map(|e| &e.me)
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

    fn send_message(&mut self, sender_id: usize, r#type: String, text: String) {
        if let Some(sender) = self.participant_by_id(sender_id) {
            if sender.username.is_none() || sender.display.is_none() {
                return;
            }
            let now = Utc::now();
            let username = sender.username.as_ref().unwrap();
            let display = sender.display.as_ref().unwrap();

            self.broadcast_json(&TextRoomEvent::Message {
                from: username,
                display,
                date: now,
                text: &text,
                r#type: &r#type,
            });

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

            self.messages += 1;
        }
    }
    fn announce(&mut self, from_sender_id: usize, r#type: String, text: String) {
        if let Some(sender) = self
            .participant_by_id(from_sender_id)
            .map(|e| e.username.as_deref())
            .flatten()
        {
            let now = Utc::now();
            self.broadcast_json(&TextRoomEvent::Announcement {
                date: now,
                text: &text,
                r#type: &r#type,
            });

            self.post(
                &Message {
                    room: &self.room_name,
                    textroom: Message::ANNOUNCEMENT,
                    r#type: &r#type,
                    text: &text,
                    date: now,
                    from: sender,
                },
                true,
            );

            self.messages += 1;
            self.last_announcements.insert(r#type, text);
        }
    }

    fn ban(&mut self, sender_id: usize, victim: String) {
        if let Some(from) = self
            .participant_by_id(sender_id)
            .map(|e| e.username.as_deref())
            .flatten()
        {
            log!("{from} wants to ban {victim}");

            self.post(
                &Message {
                    textroom: Message::MODERATE,
                    room: &self.room_name,
                    r#type: Message::TYPE_BAN.into(),
                    text: &victim,
                    date: Utc::now(),
                    from,
                },
                false,
            );
            let victims: Vec<usize> = self
                .clients
                .iter()
                .filter(|(_, client)| client.me.username.eq_to_some(&victim))
                .map(|(id, _)| id.to_owned())
                .collect();
            let event = serde_json::to_string(&TextRoomEvent::Banned).unwrap();
            for id in victims {
                if let Some(client) = self.leave(id) {
                    client.op.spawn().Send(WsMessage::Text(event.clone()));
                }
            }
        }
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
        for (_, client) in &self.clients {
            client.op.spawn().Send(WsMessage::Text(body.clone()))
        }
    }

    fn reply_json<T: Serialize>(&self, receiver_id: usize, body: &T) {
        if let Some(client) = self.clients.get(&receiver_id) {
            client
                .op
                .spawn()
                .Send(WsMessage::Text(serde_json::to_string(body).unwrap()));
        }
    }
    fn destroy(&mut self) {
        if !self.is_destroyed {
            self.is_destroyed = true;
            self.broadcast_json(&TextRoomEvent::Destroyed);
            self.clients.clear();
            log!("room `{}` destroyed", self.room_name);
            self.post(&Message::room_destroyed(&self.room_name), false)
        }
    }
}

#[derive(Deserialize)]
struct UnknownTextRoomRequest {
    transaction: Option<String>,
}
