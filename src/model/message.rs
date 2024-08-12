use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, Serialize)]
pub struct Message {
    pub textroom: &'static str,
    pub room: String,
    pub r#type: String,
    pub from: String,
    pub text: String,
    #[serde(with = "crate::misc::date_serde")]
    pub date: DateTime<Utc>,
}

impl Message {
    pub const MODERATE: &'static str = "moderate";
    pub const ANNOUNCEMENT: &'static str = "announcement";
    pub const MESSAGE: &'static str = "message";

    pub const TYPE_BAN: &'static str = "ban";
    pub const TYPE_ROOM_CREATED: &'static str = "room_created";
    pub const TYPE_ROOM_DESTROYED: &'static str = "room_destroyed";

    pub fn room_created(room: String) -> Message {
        Message {
            textroom: Message::MODERATE,
            room,
            r#type: Message::TYPE_ROOM_CREATED.into(),
            text: "".to_string(),
            date: Utc::now(),
            from: "".to_string(),
        }
    }

    pub fn room_destroyed(room: String) -> Message {
        Message {
            textroom: Message::MODERATE,
            room,
            r#type: Message::TYPE_ROOM_DESTROYED.into(),
            text: "".to_string(),
            date: Utc::now(),
            from: "".to_string(),
        }
    }
}
