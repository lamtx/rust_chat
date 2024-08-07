use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct Participant {
    pub username: Option<String>,
    pub display: Option<String>,
}
