use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use params::{*};
pub use request::TextRoomRequest;
pub use response::TextRoomResponse;
pub use event::TextRoomEvent;

mod response;
mod request;
mod params;
mod event;

type UtcDate = DateTime<Utc>;

#[derive(Deserialize, Clone)]
pub struct Message {
    pub textroom: String,
    pub room: String,
    pub r#type: String,
    pub text: String,
    #[serde(with = "crate::misc::date_serde")]
    pub date: DateTime<Utc>,
    pub from: String,
}

impl Message {
    pub const MODERATE: &'static str = "moderate";
    pub const ANNOUNCEMENT: &'static str = "announcement";
    pub const MESSAGE: &'static str = "message";

    pub const TYPE_BAN: &'static str = "ban";
    pub const TYPE_ROOM_CREATED: &'static str = "room_created";
    pub const TYPE_ROOM_DESTROYED: &'static str = "room_destroyed";

    pub fn room_destroyed(room: String) -> Message {
        Message {
            textroom: Message::MODERATE.to_string(),
            room,
            r#type: Message::TYPE_ROOM_DESTROYED.to_string(),
            text: "".to_string(),
            date: Utc::now(),
            from: "".to_string(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Participant {
    pub username: Option<String>,
    pub display: Option<String>,
}
