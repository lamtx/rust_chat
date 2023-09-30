use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use params::{*};
pub use request::TextRoomRequest;
pub use response::TextRoomResponse;

mod response;
mod request;
mod params;

type UtcDate = DateTime<Utc>;

#[derive(Deserialize)]
pub struct Message {
    pub textroom: String,
    pub room: String,
    pub r#type: String,
    pub text: String,
    #[serde(with = "crate::misc::date_serde")]
    pub date: DateTime<Utc>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Participant {
    pub id: i64,
    pub username: Option<String>,
    pub display: Option<String>,
}