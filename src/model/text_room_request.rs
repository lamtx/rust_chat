use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "textroom")]
pub enum TextRoomRequest {
    #[serde(rename = "announcement")]
    Announcement {
        r#type: String,
        text: String,
        secret: String,
        #[serde(skip_serializing)]
        transaction: Option<String>,
    },

    #[serde(rename = "ban")]
    Ban {
        username: String,
        secret: String,
        #[serde(skip_serializing)]
        transaction: Option<String>,
    },

    #[serde(rename = "leave")]
    Leave {
        #[serde(skip_serializing)]
        transaction: Option<String>,
    },

    #[serde(rename = "message")]
    Message {
        r#type: String,
        text: String,
        #[serde(skip_serializing)]
        transaction: Option<String>,
    },
}

impl TextRoomRequest {
    pub fn transaction(self) -> Option<String> {
        match self {
            TextRoomRequest::Announcement { transaction, .. } => transaction,
            TextRoomRequest::Ban { transaction, .. } => transaction,
            TextRoomRequest::Leave { transaction, .. } => transaction,
            TextRoomRequest::Message { transaction, .. } => transaction,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::text_room_request::TextRoomRequest;

    #[test]
    fn it_works() {
        let obj = TextRoomRequest::Announcement {
            r#type: "message".to_string(),
            text: "Hello there!".to_string(),
            secret: "1q2w3e!@#$".to_string(),
            transaction: None,
        };
        let json = serde_json::to_string(&obj).unwrap();
        let result = r#"{"textroom":"announcement","type":"message","text":"Hello there!","secret":"1q2w3e!@#$"}"#;
        println!("json: {json}");
        let parsed = serde_json::from_str::<TextRoomRequest>(&result).unwrap();
        println!("obj: {:?}", parsed);
        assert_eq!(json, result);
        assert_eq!(obj, parsed);
    }

    #[test]
    fn test_transaction() {
        let transaction = "b6469c3f31653d281bbbfa6f94d60fea130abe38";
        let obj = TextRoomRequest::Leave {
            transaction: Some(transaction.to_string()),
        };
        let json = serde_json::to_string(&obj).unwrap();
        println!("json: {}", &json);
        assert_eq!(json, r#"{"textroom":"leave"}"#);
        let raw = format!(r#"{{"textroom":"leave","transaction":"{}"}}"#, transaction);
        let parsed = serde_json::from_str::<TextRoomRequest>(&raw).unwrap();
        println!("obj: {:?}", parsed);
        assert_eq!(obj, parsed);
    }
}
