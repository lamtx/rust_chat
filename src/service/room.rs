use crate::misc::StringExt;

pub struct Room {
    pub uid: String,
    pub secret: String,
    pub post: Option<String>,
    pub post_types: Vec<String>,
}

impl Room {
    pub fn name(&self) -> &str {
        self.uid.as_str().substring_after_last('/')
    }
}