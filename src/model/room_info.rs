use serde::Serialize;

use crate::model::Participant;

#[derive(Debug, Serialize)]
pub struct RoomInfo {
    pub room: String,
    pub participants: Vec<Participant>,
    pub messages: usize,
}
