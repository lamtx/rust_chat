use std::collections::HashMap;
use chrono::Utc;
use serde::Serialize;
use tokio::sync::mpsc::channel;

use crate::{command, error};
use crate::misc::*;
use crate::model::{JoinParams, Message, Participant, RoomInfo, TextRoomEvent};
use crate::service::client_service::{ChatClient};
use crate::service::Room;

command! {
    pub Status() -> RoomInfo,
    pub Count() -> usize,
    pub AddClient(client: ChatClient),
    pub RemoveClient(id: usize),
    pub Announcement(sender: Participant, r#type: String, text: String),
    pub Ban(from: Option<String>,victim: String),
    pub Broadcast(message: String),
    pub GetNextId()  -> usize,
    pub LastAnnouncement(types: Vec<String>) -> HashMap<String, String>,
    pub Participants() -> Vec<Participant>,
    /// Return the serialized json of all [Message] in this room.
    pub Messages() -> String,
    pub Destroy(),
}

#[derive(Clone)]
pub struct ChatRoom {
    pub secret: String,
    pub tx: CommandSender,
}

impl ChatRoom {
    pub fn create(room: Room) -> ChatRoom {
        let (tx, mut rx) = channel::<Command>(30);
        let chat_room = ChatRoom {
            tx: CommandSender { tx },
            secret: room.secret.clone(),
        };
        tokio::spawn(async move {
            let mut state = ChatRoomInner {
                room,
                clients: Vec::new(),
                messages: Vec::new(),
                next_id: 0,
                detached: false,
            };
            while let Some(command) = rx.recv().await {
                match command {
                    Command::Status { resp_tx } => {
                        resp_tx.send(state.status()).unwrap();
                    }
                    Command::Count { resp_tx } => {
                        resp_tx.send(state.count()).unwrap();
                    }
                    Command::AddClient { client, resp_tx } => {
                        let id = client.id;
                        state.clients.push(client);
                        println!("Room: client `{id}` added (size: {})", state.clients.len());
                        resp_tx.send(()).unwrap();
                    }
                    Command::Announcement { sender, r#type, text, resp_tx } => {
                        state.announcement(sender, r#type, text);
                        resp_tx.send(()).unwrap();
                    }
                    Command::Ban { from, victim, resp_tx } => {
                        state.ban(from, victim);
                        resp_tx.send(()).unwrap();
                    }
                    Command::Broadcast { message, resp_tx } => {
                        state.broadcast(message);
                        resp_tx.send(()).unwrap();
                    }
                    Command::GetNextId { resp_tx } => {
                        state.next_id += 1;
                        resp_tx.send(state.next_id).unwrap();
                    }
                    Command::RemoveClient { id, resp_tx } => {
                        state.clients.retain(|e| e.id != id);
                        println!("Room: client {id} removed (size: {})", state.clients.len());
                        resp_tx.send(()).unwrap();
                    }
                    Command::Destroy { resp_tx } => {
                        state.destroy();
                        resp_tx.send(()).unwrap();
                    }
                    Command::LastAnnouncement { types, resp_tx } => {
                        resp_tx.send(state.last_announcement(types)).unwrap()
                    }
                    Command::Participants { resp_tx } => {
                        resp_tx.send(state.participants()).unwrap();
                    }
                    Command::Messages { resp_tx } => {
                        let json = serde_json::to_string(&state.messages).unwrap();
                        resp_tx.send(json).unwrap();
                    }
                }
            }
        });
        chat_room
    }

    pub async fn join(&self, req: HttpRequest, params: JoinParams) -> AppResult<HttpResponse> {
        use hyper_tungstenite::*;
        if is_upgrade_request(&req) {
            let (response, websocket) = upgrade(req, None)
                .to_bad_request()?;
            let this = self.clone();
            tokio::spawn(async move {
                let id = this.tx.GetNextId().await;
                let me = Participant {
                    username: params.username,
                    display: params.display,
                };
                let client = ChatClient::create(websocket, this.clone(), me, id).await;
                if let Ok(client) = client {
                    this.tx.AddClient(client).await;
                } else {
                    panic!("unable to create client?")
                }
            });
            // Return the response so the spawned future can continue.
            Ok(response)
        } else {
            error!("The request is not upgradable to web socket.".to_string())
        }
    }
    
    pub async fn broadcast(&self, event: TextRoomEvent) {
        let message = serde_json::to_string(&event).unwrap();
        self.tx.Broadcast(message).await
    }
}

struct ChatRoomInner {
    room: Room,
    messages: Vec<Message>,
    clients: Vec<ChatClient>,
    next_id: usize,
    detached: bool,
}

impl ChatRoomInner {
    fn status(&self) -> RoomInfo {
        RoomInfo {
            room: self.room.uid.to_string(),
            participants: self.clients
                .iter().map(|e| e.me.clone())
                .collect(),
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
            let message = self.messages.iter().rfind(
                |x| x.textroom == Message::ANNOUNCEMENT && x.r#type == r#type
            );
            if let Some(text) = message {
                result.insert(r#type, text.r#type.clone());
            }
        }

        result
    }

    fn announcement(&mut self, sender: Participant, r#type: String, text: String) {
        let now = Utc::now();
        let message = Message {
            room: self.room.name().to_string(),
            textroom: Message::ANNOUNCEMENT.to_string(),
            r#type: r#type.clone(),
            text: text.clone(),
            date: now.clone(),
            from: sender.username.unwrap(),
        };
        self.messages.push(message.clone());
        self.post(&message, Some(&r#type));
        let event = TextRoomEvent::Announcement { date: now, text, r#type };
        self.broadcast_json(&event);
    }

    fn ban(&mut self, from: Option<String>, victim: String) {
        println!("{:?} wants to ban {victim}", from);
        let victims = self.clients.iter().filter(
            |&e| e.me.username.contains(&victim)
        );
        let event = serde_json::to_string(&TextRoomEvent::Banned).unwrap();
        for client in victims {
            client.tx.Send(event.clone());
            client.tx.Leave();
        }
        self.post(&Message {
            textroom: Message::MODERATE.to_string(),
            room: self.room.name().to_string(),
            r#type: Message::TYPE_BAN.to_string(),
            text: victim,
            date: Utc::now(),
            from: from.unwrap_or_default(),
        }, None);
    }

    fn post(&mut self, message: &Message, r#type: Option<&str>) {
        // FIXME
    }

    async fn broadcast_json<T: Serialize>(&self, body: &T) {
        let content = serde_json::to_string(body).unwrap();
        self.broadcast(content)
    }
    fn broadcast(&self, body: String) {
        for client in &self.clients {
            client.spawn_send(body.clone())
        }
    }

    async fn close(&mut self) {
        for client in &self.clients {
            client.tx.Close().await;
        }
        self.clients.clear();
        self.detach();
    }

    fn detach(&mut self) {
        if !self.detached {
            self.detached = true;
            self.broadcast_json(&TextRoomEvent::Destroyed);
            self.clients.clear();
            // FIXME
            //service.detachRoom(this);
            println!("room '{}' destroyed", self.room.name())
        }
    }
    fn destroy(&mut self) {
        self.detach();
        self.post(&Message::room_destroyed(self.room.name().to_string()), None)
    }
}