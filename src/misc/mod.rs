use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response};

pub use option_ext::OptionExt;
pub use query_params::{Params, ParseParamError, QueryParams};
pub use response::*;
pub use string_ext::{OrEmpty, StringExt};

pub mod date_serde;

mod command;
mod option_ext;
mod query_params;
mod response;
mod string_ext;

pub type HttpRequest = Request<Incoming>;

pub type HttpResponse = Response<Full<Bytes>>;
