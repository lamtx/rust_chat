use std::ops::Deref;

use futures::{sink::SinkExt, stream::StreamExt};
use futures::stream::SplitSink;
use hyper_tungstenite::{HyperWebsocket, HyperWebsocketStream};
use hyper_tungstenite::tungstenite::{Error, Message};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::command;
use crate::misc::OrEmpty;
use crate::model::{Participant, TextRoomEvent, TextRoomRequest, TextRoomResponse};
use crate::service::ChatRoom;

pub struct ChatClient {
    pub id: usize,
    pub me: Participant,
    pub op: CommandSender,
}

impl ChatClient {
    pub async fn create(
        socket: HyperWebsocket,
        room: ChatRoom,
        me: Participant,
        my_id: usize,
    ) -> Result<ChatClient, Error> {
        let (sink, mut stream) = socket.await?.split();
        let (tx, mut rx) = mpsc::channel::<Command>(30);
        let mut state = ClientImpl {
            room,
            me: me.clone(),
            my_id,
            op: Some(CommandSender { tx: tx.clone() }), //loop here
            is_detached: false,
            is_socket_closed: false,
            sink,
        };

        // command listener
        tokio::spawn(async move {
            while let Some(command) = rx.recv().await {
                match command {
                    Command::OnListen { text, resp_tx } => {
                        state.on_listen(text);
                        state.send(Message::Ping(vec![1])).await.unwrap();
                        resp_tx.send(()).unwrap()
                    }
                    Command::Send { message, resp_tx } => {
                        match state.send(message).await {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("send ws failed: {:?}", e);
                                state.close().await;
                            }
                        }
                        resp_tx.send(()).unwrap();
                    }
                    Command::Leave { resp_tx } => {
                        state.leave();
                        resp_tx.send(()).unwrap()
                    }
                    Command::Close { resp_tx } => {
                        state.close().await;
                        resp_tx.send(()).unwrap()
                    }
                }
            }
        });
        // listening to ws stream
        let op = CommandSender { tx: tx.clone() };
        tokio::spawn(async move {
            while let Some(message) = stream.next().await {
                match message {
                    Err(e) => {
                        println!("Socket error: {:?}", e);
                        break;
                    }
                    Ok(message) => match message {
                        Message::Text(text) => {
                            op.OnListen(text).await;
                        }
                        Message::Binary(msg) => {
                            println!("Received binary message: {:02X?}", msg);
                        }
                        Message::Ping(msg) => {
                            println!("Received ping message: {:02X?}", msg);
                            op.spawn().Send(Message::Pong(msg));
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
            println!("web socket ended correctly");
            op.Close().await;
        });

        Ok(ChatClient {
            id: my_id,
            op: CommandSender { tx },
            me,
        })
    }
}

pub struct ClientImpl {
    room: ChatRoom,
    me: Participant,
    my_id: usize,
    op: Option<CommandSender>,
    sink: SplitSink<HyperWebsocketStream, Message>,
    is_detached: bool,
    is_socket_closed: bool,
}

command! {
    OnListen(text: String);
    pub Close();
    pub Send(message: Message);
    pub Leave();
}

impl ClientImpl {
    fn on_listen(&mut self, message: String) {
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
            match &self.op {
                None => {}
                Some(op) => {
                    op.spawn().Send(Message::Text(serde_json::to_string(&response).unwrap()));
                }
            }
        }
    }

    fn handle_request(&mut self, request: TextRoomRequest) -> Option<TextRoomResponse> {
        if self.is_detached {
            return Some(TextRoomResponse::destroyed(request.transaction()));
        }
        println!("handling: {:?}", &request);
        match request {
            TextRoomRequest::Message { r#type, text, .. } => {
                self.room.op.spawn().SendMessage(self.me.clone(), r#type, text);
                None
            }
            TextRoomRequest::Announcement { secret, r#type, text, transaction, .. } => {
                if self.room.secret.deref() == &secret {
                    self.room.op.spawn().Announce(self.me.clone(), r#type, text);
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
                    self.room.op.spawn().Ban(self.me.username.clone(), username);
                    None
                } else {
                    Some(TextRoomResponse::secret(transaction))
                }
            }
        }
    }

    async fn send(&mut self, message: Message) -> Result<(), Error> {
        if self.is_socket_closed || self.is_detached {
            return Ok(());
        }
        self.sink.send(message).await
    }

    fn leave(&mut self) {
        self.detach();
        let room = self.room.op.clone();
        let me = self.me.clone();
        tokio::spawn(async move {
            let event = &TextRoomEvent::Left {
                username: me.username.as_deref(),
                display: me.display.as_deref(),
                participants: room.Count().await,
            };
            room.Broadcast(serde_json::to_string(event).unwrap()).await;
        });
    }
    async fn close(&mut self) {
        self.detach();
        self.is_socket_closed = true;
        let _ = self.sink.close().await;
    }

    fn detach(&mut self) {
        if !self.is_detached {
            self.is_detached = true;
            self.op = None;
            println!("`{}` left (id:{})", self.me.display.or_empty(), self.my_id);
            self.room.op.spawn().RemoveClient(self.my_id);
        }
    }
}

#[derive(Deserialize)]
struct UnknownTextRoomRequest {
    transaction: Option<String>,
}