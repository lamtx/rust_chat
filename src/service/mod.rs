pub use chat_service::ChatService;
pub(crate) use room::Room;
pub use room_service::ChatRoom;
pub use service_error::ServiceError;

mod room_service;
mod chat_service;
mod client_service;
mod room;
mod service_error;

