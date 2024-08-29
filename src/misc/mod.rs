use axum::extract::ws::{Message, WebSocket};
use futures::stream::{SplitSink, SplitStream};
use http_body_util::Full;
use hyper::{Request, Response};
use hyper::body::{Bytes, Incoming};

pub use option_ext::OptionExt;
pub use query_params::{Params, ParseParamError, QueryParams};
pub use string_ext::{OrEmpty, StringExt};

pub mod date_serde;

mod command;
mod log;
mod option_ext;
mod query_params;
mod response;
mod string_ext;

pub type HttpRequest = Request<Incoming>;

pub type HttpResponse = Response<Full<Bytes>>;

pub type WebSocketSink = SplitSink<WebSocket, Message>;
pub type WebSocketStream = SplitStream<WebSocket>;
