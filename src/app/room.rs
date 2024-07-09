use chrono::{Utc};
use serde::{Serialize};
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::oneshot;
use crate::app::client::{Client, ClientParam};
use crate::error;
use crate::misc::{*};
use crate::model::{DestroyParams, JoinParams, Message, Participant, TextRoomEvent};


#[derive(Clone)]
pub struct Room {
    commands: Commands,
    pub secret: String,
}

#[derive(Clone)]
struct Commands {
    tx: Sender<Command>,
}

impl Commands {
    pub async fn status(&self) -> RoomInfo {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.send(Command::Status { resp_tx }).await.unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn add_client(&self, client: Client) {
        self.tx.send(Command::AddClient { client }).await.unwrap()
    }

    pub async fn remove_client(&self, id: usize) {
        self.tx.send(Command::RemoveClient { id }).await.unwrap()
    }

    pub async fn announcement(
        &self,
        sender: Participant,
        r#type: String,
        text: String,
    ) {
        self.tx.send(Command::Announcement { sender, r#type, text }).await.unwrap()
    }

    pub async fn ban(&self, from: Option<String>, victim: String) {
        self.tx.send(Command::Ban { from, victim }).await.unwrap()
    }

    pub async fn broadcast(&self, p0: String) {
        self.tx.send(Command::Broadcast(p0)).await.unwrap()
    }

    pub async fn get_next_id(&self) -> usize {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.send(Command::GetNextId { resp_tx }).await.unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn destroy(&self) {
        self.tx.send(Command::Destroy).await.unwrap()
    }
}

#[derive(Default)]
pub struct RoomParams {
    pub room: String,
    pub secret: String,
    pub post: Option<String>,
    pub post_types: Option<Vec<String>>,
}

struct RoomImpl {
    params: RoomParams,
    messages: Vec<Message>,
    clients: Vec<Client>,
    next_id: usize,
    room_name: String,
    detached: bool,
}

#[derive(Debug, Serialize)]
pub struct RoomInfo {
    pub room: String,
    pub participants: Vec<Participant>,
    pub messages: usize,
}

enum Command {
    Status {
        resp_tx: oneshot::Sender<RoomInfo>
    },
    AddClient {
        client: Client,
    },
    RemoveClient {
        id: usize,
    },
    Announcement {
        sender: Participant,
        r#type: String,
        text: String,
    },
    Ban {
        from: Option<String>,
        victim: String,
    },
    Broadcast(String),
    GetNextId {
        resp_tx: oneshot::Sender<usize>
    },
    Destroy,
}

impl RoomParams {
    pub fn create(self) -> Room {
        let (tx, mut rx) = channel::<Command>(30);
        let room = Room {
            commands: Commands { tx },
            secret: self.secret.clone(),
        };
        let room_name = self.room.as_str().substring_after_last('/').to_string();
        tokio::spawn(async move {
            let mut state = RoomImpl {
                params: self,
                clients: Default::default(),
                messages: Default::default(),
                next_id: 0,
                room_name,
                detached: false,
            };
            while let Some(command) = rx.recv().await {
                match command {
                    Command::Status { resp_tx } => {
                        resp_tx.send(state.status()).unwrap();
                    }
                    Command::AddClient { client } => {
                        let id = client.id;
                        state.clients.push(client);
                        println!("Room: client `{id}` added (size: {})", state.clients.len())
                    }
                    Command::Announcement { sender, r#type, text } => {
                        state.announcement(sender, r#type, text);
                    }
                    Command::Ban { from, victim } => {
                        state.ban(from, victim);
                    }
                    Command::Broadcast(content) => {
                        state.broadcast(content)
                    }
                    Command::GetNextId { resp_tx } => {
                        state.next_id += 1;
                        resp_tx.send(state.next_id).unwrap();
                    }
                    Command::RemoveClient { id } => {
                        state.clients.retain(|e| e.id != id);
                        println!("Room: client {id} removed (size: {})", state.clients.len())
                    }
                    Command::Destroy => state.destroy(),
                }
            }
        });
        room
    }
}

impl Room {
    pub async fn handle_request(&self, req: HttpRequest, action: String) -> AppResult<HttpResponse> {
        match action.as_str() {
            "join" => {
                let params = Params::parse_uri(req.uri())?;
                self.join(req, params).await
            }
            "destroy" => {
                let params: DestroyParams = Params::parse_uri(req.uri())?;
                if params.secret == self.secret {
                    self.commands.destroy().await;
                    Ok(ok_response())
                } else {
                    Err(AppError::secret())
                }
            }
            "status" => Ok(self.status().await.to_response()),
            _ => not_found()
        }
    }

    pub fn post_created(&self) {
        // FIXME
    }

    pub async fn status(&self) -> RoomInfo {
        self.commands.status().await
    }

    async fn join(&self, req: HttpRequest, params: JoinParams) -> AppResult<HttpResponse> {
        use hyper_tungstenite::*;
        if is_upgrade_request(&req) {
            let (response, websocket) = upgrade(req, None)
                .to_bad_request()?;
            let this = self.clone();
            tokio::spawn(async move {
                let id = this.commands.get_next_id().await;

                let client = ClientParam {
                    id,
                    me: Participant {
                        username: params.username,
                        display: params.display,
                    },
                    room: this.clone(),
                }.listen_to(websocket).await;

                if let Ok(client) = client {
                    this.commands.add_client(client).await;
                }
            });
            // Return the response so the spawned future can continue.
            Ok(response)
        } else {
            error!("The request is not upgradable to web socket.".to_string())
        }
    }

    pub fn remove(&self, client_id: usize) {
        let commands = self.commands.clone();
        tokio::spawn(async move {
            commands.remove_client(client_id).await;
        });
    }

    pub fn announcement(&self, sender: Participant, r#type: String, text: String) {
        if sender.username.is_none() {
            return;
        }
        let commands = self.commands.clone();
        tokio::spawn(async move {
            commands.announcement(sender, r#type, text).await;
        });
    }

    pub fn ban(&self, from: Option<String>, victim: String) {
        let tx = self.commands.clone();
        tokio::spawn(async move {
            tx.ban(from, victim).await
        });
    }

    pub fn broadcast(&self, event: TextRoomEvent) {
        let body = serde_json::to_string(&event).unwrap();
        let tx = self.commands.clone();
        tokio::spawn(async move {
            tx.broadcast(body).await
        });
    }
}

impl RoomImpl {
    fn status(&self) -> RoomInfo {
        RoomInfo {
            room: self.params.room.to_string(),
            participants: self.clients
                .iter().map(|e| e.me.clone())
                .collect(),
            messages: self.messages.len(),
        }
    }

    fn announcement(&mut self, sender: Participant, r#type: String, text: String) {
        let now = Utc::now();
        let message = Message {
            room: self.params.room.as_str().substring_after_last('/').to_string(),
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
        let victims = self.clients.iter().filter(|&e|
            e.me.username.contains(&victim)
        );
        let event = serde_json::to_string(&TextRoomEvent::Banned).unwrap();
        for client in victims {
            client.send(event.clone());
            client.leave();
        }
        self.post(&Message {
            textroom: Message::MODERATE.to_string(),
            room: self.room_name.clone(),
            r#type: Message::TYPE_BAN.to_string(),
            text: victim,
            date: Utc::now(),
            from: from.unwrap_or_default(),
        }, None);
    }

    fn post(&mut self, message: &Message, r#type: Option<&str>) {
        // FIXME
    }

    fn broadcast_json<T: Serialize>(&self, body: &T) {
        let content = serde_json::to_string(body).unwrap();
        self.broadcast(content)
    }
    fn broadcast(&self, body: String) {
        for client in &self.clients {
            client.send(body.clone())
        }
    }

    async fn close(&mut self) {
        for client in &self.clients {
            client.close().await;
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
            println!("room '{}' destroyed", self.room_name)
        }
    }
    fn destroy(&mut self) {
        self.detach();
        self.post(&Message::room_destroyed(self.room_name.clone()), None)
    }
}