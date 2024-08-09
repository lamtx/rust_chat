use hyper::{Body, Response, StatusCode};
use hyper::header::CONTENT_TYPE;

use crate::misc::HttpResponse;

pub fn ok_response() -> HttpResponse {
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())
        .unwrap()
}

pub fn json_response(json: String) -> HttpResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(json))
        .unwrap()
}

#[macro_export]
macro_rules! json_response {
    ($($json:tt)+) => {{
        let value = json!($($json)+);
        let json_body = serde_json::to_string(&value).unwrap();
        json_response(json_body)
    }};
}