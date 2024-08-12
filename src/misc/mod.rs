use http_body_util::Full;
use hyper::{Request, Response};
use hyper::body::{Bytes, Incoming};

pub use option_ext::OptionExt;
pub use query_params::{Params, ParseParamError, QueryParams};
pub use response::*;
pub use string_ext::{OrEmpty, StringExt};

pub mod date_serde;

mod string_ext;
mod query_params;
mod response;
mod option_ext;
mod command;

// pub type AppResult<T> = Result<T, AppError>;

pub type HttpRequest = Request<Incoming>;

pub type HttpResponse = Response<Full<Bytes>>;

