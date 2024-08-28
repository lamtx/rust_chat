use std::ops::Deref;

use futures::stream::{SplitSink, SplitStream};
use futures::{sink::SinkExt, stream::StreamExt, TryStreamExt};
use hyper_tungstenite::tungstenite::{Error, Message};
use hyper_tungstenite::{HyperWebsocket, HyperWebsocketStream};
use serde::Deserialize;
use tokio::task::JoinHandle;
use tokio::time::{interval, Instant};

use crate::config::PING_INTERVAL;
use crate::misc::OrEmpty;
use crate::model::{Participant, TextRoomEvent, TextRoomRequest, TextRoomResponse};
use crate::service::ChatRoom;
use crate::{command, log};

pub struct ChatClient {
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
        let (sink, stream) = socket.await?.split();
        let (op, mut rx) = Command::new_channel();
        let socket_stream_token = Self::listen_to_socket(&op, stream);
        let ping_token = Self::ping_repeatedly(&op);

        let mut state = Box::new(ClientInner {
            room,
            me: me.clone(),
            my_id,
            op: Some(op.clone()),
            is_detached: false,
            is_socket_closed: false,
            last_pong: Instant::now(),
            ping_token,
            socket_stream_token,
            sink,
        });
        // command listener
        tokio::spawn(async move {
            while let Some(command) = rx.recv().await {
                match command {
                    Command::OnMessageReceived { message, resp_tx } => {
                        state.on_message_received(message).await;
                        let _ = resp_tx.send(());
                    }
                    Command::Send { message, resp_tx } => {
                        state.send(message).await;
                        let _ = resp_tx.send(());
                    }
                    Command::Leave { resp_tx } => {
                        state.leave();
                        let _ = resp_tx.send(());
                    }
                    Command::Close { resp_tx } => {
                        state.close().await;
                        let _ = resp_tx.send(());
                    }
                    Command::SendPing { instant, resp_tx } => {
                        let _ = resp_tx.send(state.send_ping(instant).await);
                    }
                }
            }
            log!("client `{}` dropped", state.me.display.or_empty())
        });

        Ok(ChatClient { op, me })
    }

    fn listen_to_socket(
        op: &CommandSender,
        mut stream: SplitStream<HyperWebsocketStream>,
    ) -> JoinHandle<()> {
        let op = op.clone();
        tokio::spawn(async move {
            while let Some(message) = stream.try_next().await.ok().flatten() {
                op.OnMessageReceived(message).await;
            }
            log!("web socket's stream ended");
            op.Close().await;
        })
    }

    fn ping_repeatedly(op: &CommandSender) -> JoinHandle<()> {
        let op = op.clone();
        tokio::spawn(async move {
            let mut interval = interval(PING_INTERVAL);
            loop {
                let instant = interval.tick().await;
                if !op.SendPing(instant).await {
                    log!("client closed, stop sending ping");
                    break;
                }
            }
        })
    }
}

pub struct ClientInner {
    room: ChatRoom,
    me: Participant,
    my_id: usize,
    // weak reference, set null of client is closed
    op: Option<CommandSender>,
    sink: SplitSink<HyperWebsocketStream, Message>,
    last_pong: Instant,
    ping_token: JoinHandle<()>,
    socket_stream_token: JoinHandle<()>,
    is_detached: bool,
    is_socket_closed: bool,
}

command! {
    OnMessageReceived(message: Message);
    SendPing(instant: Instant) -> bool;
    pub Close();
    pub Send(message: Message);
    pub Leave();
}

impl ClientInner {
    async fn on_message_received(&mut self, message: Message) {
        match message {
            Message::Text(text) => {
                self.on_listen(text);
            }
            Message::Binary(msg) => {
                log!("unexpected binary message: {:02X?}", msg);
            }
            Message::Ping(_) => {}
            Message::Pong(_) => {
                let now = Instant::now();
                log!("received pong at {:?}", now);
                self.last_pong = now;
            }
            Message::Close(msg) => {
                if let Some(msg) = &msg {
                    log!(
                        "received close message with code {} and message: {}",
                        msg.code,
                        msg.reason
                    );
                } else {
                    log!("Received close message");
                }
            }
            Message::Frame(_) => {}
        }
    }

    fn on_listen(&mut self, message: String) {
        log!("receive: {message}");

        let response = match serde_json::from_str(&message) {
            Ok(value) => self.handle_request(value),
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
            match &self.op {
                None => {}
                Some(op) => {
                    op.spawn()
                        .Send(Message::Text(serde_json::to_string(&response).unwrap()));
                }
            }
        }
    }

    fn handle_request(&mut self, request: TextRoomRequest) -> Option<TextRoomResponse> {
        if self.is_detached {
            return Some(TextRoomResponse::destroyed(request.transaction()));
        }
        log!("handling ws message: {:?}", &request);
        match request {
            TextRoomRequest::Message { r#type, text, .. } => {
                self.room
                    .op
                    .spawn()
                    .SendMessage(self.me.clone(), r#type, text);
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
            TextRoomRequest::Ban {
                secret,
                username,
                transaction,
            } => {
                if self.room.secret.deref() == &secret {
                    self.room.op.spawn().Ban(self.me.username.clone(), username);
                    None
                } else {
                    Some(TextRoomResponse::secret(transaction))
                }
            }
        }
    }

    async fn send(&mut self, message: Message) {
        if self.is_socket_closed || self.is_detached {
            return;
        }
        async fn send_and_flush(
            sink: &mut SplitSink<HyperWebsocketStream, Message>,
            message: Message,
        ) -> Result<(), Error> {
            sink.send(message).await?;
            sink.flush().await?;
            Ok(())
        }
        match send_and_flush(&mut self.sink, message).await {
            Ok(_) => {}
            Err(e) => {
                log!("send ws failed: {:?}", e);
                self.close().await;
            }
        }
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
    async fn send_ping(&mut self, instant: Instant) -> bool {
        if self.is_socket_closed {
            return false;
        }
        let duration = instant.duration_since(self.last_pong);
        log!("last pong since {:?}", duration);
        let responded = duration <= PING_INTERVAL * 2;
        if responded {
            log!("send ping at {:?}", instant);
            self.send(Message::Ping(const { Vec::new() })).await;
        } else {
            log!(
                "client `{}` not responded, closing",
                self.me.display.or_empty()
            );
            self.close().await
        }
        responded
    }

    async fn close(&mut self) {
        if !self.is_socket_closed {
            self.detach();
            self.is_socket_closed = true;
            self.ping_token.abort();
            self.socket_stream_token.abort();
            let _ = self.sink.close().await;
        }
    }

    fn detach(&mut self) {
        if !self.is_detached {
            self.is_detached = true;
            self.op = None;
            log!(
                "client `{}` left (id:{})",
                self.me.display.or_empty(),
                self.my_id
            );
            self.room.op.spawn().RemoveClient(self.my_id);
        }
    }
}

#[derive(Deserialize)]
struct UnknownTextRoomRequest {
    transaction: Option<String>,
}
