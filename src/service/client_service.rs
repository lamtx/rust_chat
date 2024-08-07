use futures::{sink::SinkExt, stream::StreamExt};
use hyper_tungstenite::HyperWebsocket;
use hyper_tungstenite::tungstenite::Message;
use tokio::sync::mpsc;
use crate::model::{Participant, TextRoomEvent, TextRoomRequest, TextRoomResponse};
use crate::misc::{AppResult, ToBadRequest};
use crate::service::ChatRoom;

#[derive(Clone)]
pub struct ChatClient {
    pub id: usize,
    pub me: Participant,
    tx: mpsc::Sender<Command>,
}

pub struct ClientParam {
    pub room: ChatRoom,
    pub me: Participant,
    pub id: usize,
}

pub struct ClientImpl {
    params: ClientParam,
    tx: mpsc::Sender<Command>,
}

enum Command {
    OnListen(String),
    Close,
    Send {
        body: String,
    },
    Leave,
}

impl ClientParam {
    pub async fn listen_to(self, socket: HyperWebsocket) -> AppResult<ChatClient> {
        let (mut sink, mut stream) = socket.await.to_bad_request()?.split();
        let (tx, mut rx) = mpsc::channel::<Command>(30);
        let client = ChatClient {
            id: self.id,
            tx: tx.clone(),
            me: self.me.clone(),
        };
        let mut state = ClientImpl { params: self, tx: tx.clone() };
        tokio::spawn(async move {
            while let Some(command) = rx.recv().await {
                match command {
                    Command::OnListen(text) => {
                        state.on_listen(text);
                    }
                    Command::Send { body } => {
                        sink.send(Message::Text(body)).await.unwrap();
                    }
                    Command::Leave => state.leave(),
                    Command::Close => state.close(),
                }
            }
        });
        let tx2 = tx;
        tokio::spawn(async move {
            while let Some(message) = stream.next().await {
                match message {
                    Err(_) => break,
                    Ok(message) => match message {
                        Message::Text(text) => tx2.send(Command::OnListen(text)).await.unwrap(),
                        Message::Binary(msg) => {
                            println!("Received binary message: {:02X?}", msg);
                        }
                        Message::Ping(msg) => {
                            println!("Received ping message: {:02X?}", msg);
                        }
                        Message::Pong(msg) => {
                            println!("Received pong message: {:02X?}", msg);
                        }
                        Message::Close(msg) => {
                            if let Some(msg) = &msg {
                                println!("Received close message with code {} and message: {}", msg.code, msg.reason);
                            } else {
                                println!("Received close message");
                            }
                        }
                        Message::Frame(_) => {
                            unreachable!();
                        }
                    }
                }
            }
            tx2.send(Command::Close).await.unwrap();
        });
        Ok(client)
    }
}

impl ChatClient {
    pub fn send(&self, body: String) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            tx.send(Command::Send { body }).await.unwrap();
        });
    }

    pub fn leave(&self) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            tx.send(Command::Leave).await.unwrap();
        });
    }

    pub async fn close(&self) {
        self.tx.send(Command::Close).await.unwrap();
    }
}

impl ClientImpl {
    fn on_listen(&self, message: String) {
        println!("receive: {message}");
        let response = match serde_json::from_str::<TextRoomRequest>(&message) {
            Ok(value) => {
                self.handle_request(value)
            }
            Err(e) => {
                println!("parse json failed: {:?}", e);
                // FIXME: how to pass transaction back
                Some(TextRoomResponse::Error {
                    transaction: None,
                    error: "None".to_string(),
                })
            }
        };
        if let Some(response) = response {
            self.send(serde_json::to_string(&response).unwrap());
        }
    }

    fn handle_request(&self, request: TextRoomRequest) -> Option<TextRoomResponse> {
        // FIXME
        // if (room.detached || _detached) {
        //     return ErrorResponse.roomDestroyed(transaction);
        // }
        match request {
            TextRoomRequest::Announcement { secret, r#type, text, transaction, .. } => {
                if self.params.room.secret == secret {
                    self.announcement(r#type, text);
                    None
                } else {
                    Some(TextRoomResponse::secret(transaction))
                }
            }
            TextRoomRequest::Ban { secret, username, transaction } => {
                if self.params.room.secret == secret {
                    self.ban(username);
                    None
                } else {
                    Some(TextRoomResponse::secret(transaction))
                }
            }
            TextRoomRequest::Leave { transaction } => {
                self.leave();
                Some(TextRoomResponse::left(transaction))
            }
            TextRoomRequest::Message { .. } => {
                None
            }
        }
    }

    fn send(&self, body: String) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            tx.send(Command::Send { body }).await.unwrap();
        });
    }

    fn announcement(&self, r#type: String, text: String) {
        let room = self.params.room.clone();
        let me = self.params.me.clone();
        tokio::spawn(async move {
            room.tx.Announcement(me, r#type, text).await;
        });
    }

    fn ban(&self, username: String) {
        let room = self.params.room.clone();
        let me = self.params.me.username.clone();
        tokio::spawn(async move {
            room.tx.Ban(me, username).await;
        });
    }

    fn leave(&self) {
        self.detach();
        let room = self.params.room.clone();
        let event = TextRoomEvent::Left {
            username: self.params.me.username.clone(),
            display: self.params.me.display.clone(),
            participants: 0, // FIXME
        };
        tokio::spawn(async move {
            room.broadcast(event).await
        });
    }
    fn close(&mut self) {
        self.detach();
        println!("closing")
    }

    fn detach(&self) {
        println!("`{:?}` left", self.params.me.display);
        self.params.room.remove(self.params.id);
    }
}