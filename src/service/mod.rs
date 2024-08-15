pub use chat_service::ChatService;
pub use room_service::ChatRoom;
pub use service_error::ServiceError;

mod chat_service;
mod client_service;
mod rest_client;
mod room_service;
mod service_error;
