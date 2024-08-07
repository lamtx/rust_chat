use serde::Serialize;

#[derive(Serialize)]
#[serde(untagged)]
pub enum TextRoomResponse {
    Ok {
        transaction: Option<String>,
        ok: String,
    },
    Error {
        transaction: Option<String>,
        error: String,
    },
}

impl TextRoomResponse {
    pub fn left(transaction: Option<String>) -> TextRoomResponse {
        TextRoomResponse::Ok {
            transaction,
            ok: "left".to_string(),
        }
    }
    pub fn destroyed(transaction: Option<String>) -> TextRoomResponse {
        TextRoomResponse::Ok {
            transaction,
            ok: "destroyed".to_string(),
        }
    }

    pub fn secret(transaction: Option<String>) -> TextRoomResponse {
        TextRoomResponse::Error {
            transaction,
            error: "Secret does not match.".to_string(),
        }
    }

    pub fn room_destroyed(transaction: Option<String>) -> TextRoomResponse {
        TextRoomResponse::Error {
            transaction,
            error: "Room was destroyed.".to_string(),
        }
    }

    pub fn room_id(transaction: Option<String>) -> TextRoomResponse {
        TextRoomResponse::Error {
            transaction,
            error: "Not in this room.".to_string(),
        }
    }
}
