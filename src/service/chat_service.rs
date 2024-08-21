use std::collections::HashMap;

use crate::command;
use crate::model::{Room, RoomInfo};
use crate::service::{ChatRoom, ServiceError};

command! {
    pub CreateRoom(room: Room) -> Result<(), ServiceError>;
    pub Status()  -> Vec<RoomInfo>;
    pub GetRoom(room: String) -> Result<ChatRoom, ServiceError>;
    pub DetachRoom(room: String);
}

pub struct ChatService {
    pub op: CommandSender,
}

impl ChatService {
    pub fn create() -> ChatService {
        let (op, mut rx) = Command::new_channel();
        let app = ChatService { op: op.clone() };
        tokio::spawn(async move {
            use Command::*;
            let mut state = ChatServiceInner::default();

            while let Some(command) = rx.recv().await {
                match command {
                    CreateRoom { room, resp_tx } => {
                        let result = state.create_room(room, &op);
                        let _ = resp_tx.send(result);
                    }
                    Status { resp_tx } => {
                        let result = state.status().await;
                        let _ = resp_tx.send(result);
                    }
                    GetRoom { room, resp_tx } => {
                        let result = state.get_room(&room);
                        let _ = resp_tx.send(result);
                    }
                    DetachRoom { room, resp_tx } => {
                        state.detach_room(room);
                        let _ = resp_tx.send(());
                    }
                }
            }
        });

        app
    }
}

#[derive(Default)]
struct ChatServiceInner {
    rooms: HashMap<String, ChatRoom>,
}

impl ChatServiceInner {
    async fn status(&self) -> Vec<RoomInfo> {
        let mut result = Vec::new();
        for e in self.rooms.values() {
            result.push(e.op.Status().await)
        }
        result
    }

    fn create_room(&mut self, room: Room, op: &CommandSender) -> Result<(), ServiceError> {
        if self.rooms.contains_key(&room.uid) {
            Err(ServiceError::RoomNotFound)
        } else {
            let uid = room.uid.clone();
            let op = op.clone();
            let chat_room = ChatRoom::create(room, move |uid| op.spawn().DetachRoom(uid));
            self.rooms.insert(uid, chat_room);
            Ok(())
        }
    }

    fn get_room(&self, room: &str) -> Result<ChatRoom, ServiceError> {
        if let Some(instance) = self.rooms.get(room) {
            Ok(instance.clone())
        } else {
            Err(ServiceError::RoomNotFound)
        }
    }

    fn detach_room(&mut self, uid: String) {
        self.rooms.remove(&uid);
    }
}
