use std::ops::Deref;

use futures::{sink::SinkExt, stream::StreamExt};
use hyper_tungstenite::HyperWebsocket;
use hyper_tungstenite::tungstenite::{Error, Message};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::command;
use crate::model::{Participant, TextRoomEvent, TextRoomRequest, TextRoomResponse};
use crate::service::ChatRoom;

#[derive(Clone)]
pub struct ChatClient {
    pub id: usize,
    pub me: Participant,
    pub tx: CommandSender,
}

impl ChatClient {
    pub async fn create(
        socket: HyperWebsocket,
        room: ChatRoom,
        me: Participant,
        my_id: usize,
    ) -> Result<ChatClient, Error> {
        let (mut sink, mut stream) = socket.await?.split();
        let (tx, mut rx) = mpsc::channel::<Command>(30);

        let mut state = ClientImpl {
            room,
            me: me.clone(),
            my_id,
            tx: CommandSender { tx: tx.clone() },
        };
        // command listener
        tokio::spawn(async move {
            while let Some(command) = rx.recv().await {
                match command {
                    Command::OnListen { text, resp_tx } => {
                        state.on_listen(text);
                        resp_tx.send(()).unwrap()
                    }
                    Command::Send { body, resp_tx } => {
                        sink.send(Message::Text(body)).await.unwrap();
                        resp_tx.send(()).unwrap()
                    }
                    Command::Leave { resp_tx } => {
                        state.leave();
                        resp_tx.send(()).unwrap()
                    }
                    Command::Close { resp_tx } => {
                        state.close();
                        resp_tx.send(()).unwrap()
                    }
                }
            }
        });
        let sender = CommandSender { tx: tx.clone() };
        // listening to ws stream
        tokio::spawn(async move {
            while let Some(message) = stream.next().await {
                match message {
                    Err(_) => break,
                    Ok(message) => match message {
                        Message::Text(text) => {
                            sender.OnListen(text).await;
                        }
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
            sender.Close().await;
        });

        Ok(ChatClient {
            id: my_id,
            tx: CommandSender { tx },
            me,
        })
    }

    pub fn spawn_send(&self, body: String) {
        let sender = self.tx.clone();
        tokio::spawn(async move {
            sender.Send(body).await;
        });
    }
}

pub struct ClientImpl {
    room: ChatRoom,
    me: Participant,
    my_id: usize,
    tx: CommandSender,
}
command! {
    OnListen(text: String),
    pub Close(),
    pub Send(body: String),
    pub Leave(),
}

impl ClientImpl {
    fn on_listen(&self, message: String) {
        println!("receive: {message}");

        let response = match serde_json::from_str(&message) {
            Ok(value) => {
                self.handle_request(value)
            }
            Err(e) => {
                println!("parse json failed: {:?}", e);
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
            println!("reply: {:?}", response);
            self.send(serde_json::to_string(&response).unwrap());
        }
    }

    fn handle_request(&self, request: TextRoomRequest) -> Option<TextRoomResponse> {
        // FIXME
        // if (room.detached || _detached) {
        //     return ErrorResponse.roomDestroyed(transaction);
        // }
        println!("handling: {:?}", &request);
        match request {
            TextRoomRequest::Message { r#type, text, .. } => {
                self.room.send.spawn().SendMessage(self.me.clone(), r#type, text);
                None
            }
            TextRoomRequest::Announcement { secret, r#type, text, transaction, .. } => {
                if self.room.secret.deref() == &secret {
                    self.room.send.spawn().Announcement(self.me.clone(), r#type, text);
                    None
                } else {
                    Some(TextRoomResponse::secret(transaction))
                }
            }
            TextRoomRequest::Leave { transaction } => {
                self.leave();
                Some(TextRoomResponse::left(transaction))
            }
            TextRoomRequest::Ban { secret, username, transaction } => {
                if self.room.secret.deref() == &secret {
                    self.room.send.spawn().Ban(self.me.username.clone(), username);
                    None
                } else {
                    Some(TextRoomResponse::secret(transaction))
                }
            }
            _ => None
        }
    }

    fn send(&self, body: String) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            tx.Send(body).await;
        });
    }

    fn leave(&self) {
        self.detach();
        let room = self.room.send.clone();
        let me = self.me.clone();
        tokio::spawn(async move {
            let event = TextRoomEvent::Left {
                username: me.username.clone(),
                display: me.display.clone(),
                participants: room.Count().await,
            };
            room.Broadcast(serde_json::to_string(&event).unwrap()).await;
        });
    }
    fn close(&mut self) {
        self.detach();
        println!("closing")
    }

    fn detach(&self) {
        println!("`{:?}` left", self.me.display);
        self.room.send.spawn().RemoveClient(self.my_id);
    }
}

#[derive(Deserialize)]
struct UnknownTextRoomRequest {
    transaction: Option<String>,
}