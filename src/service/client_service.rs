use axum::extract::ws::Message;
use futures::sink::SinkExt;

use crate::{command, log};
use crate::misc::{OrEmpty, WebSocketSink};
use crate::model::Participant;

pub struct ChatClient {
    pub op: CommandSender,
    pub me: Participant,
}

impl ChatClient {
    pub fn new(sink: WebSocketSink, me: Participant) -> ChatClient {
        let (op, mut rx) = Command::new_channel();

        let mut inner = ClientInner { sink };
        let debug_name = me.display.or_empty().to_string();
        tokio::spawn(async move {
            while let Some(command) = rx.recv().await {
                match command {
                    Command::Send { message, resp_tx } => {
                        let _ = resp_tx.send(inner.send(message).await);
                    }
                }
            }
            log!("client `{debug_name}` dropped")
        });

        ChatClient { op, me }
    }
}

pub struct ClientInner {
    sink: WebSocketSink,
}

command! {
    pub Send(message: Message);
}

impl ClientInner {
    async fn send(&mut self, message: Message) {
        let _ = self.sink.send(message).await;
    }
}
