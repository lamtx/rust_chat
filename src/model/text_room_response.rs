use serde::Serialize;

#[derive(Serialize, Debug)]
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
        TextRoomResponse::Error {
            transaction,
            error: "Room was destroyed".to_string(),
        }
    }

    pub fn secret(transaction: Option<String>) -> TextRoomResponse {
        TextRoomResponse::Error {
            transaction,
            error: "Secret does not match.".to_string(),
        }
    }
}
