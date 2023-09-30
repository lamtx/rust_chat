use hyper::{Body, Response, StatusCode};
use hyper::header::CONTENT_TYPE;
use serde::Serialize;
use crate::misc::HttpResponse;

pub trait ToResponse {
    fn to_response(&self) -> HttpResponse;
}

pub trait ToListResponse {
    fn to_response(&self) -> HttpResponse;
}

impl<T> ToResponse for T where T: Serialize {
    fn to_response(&self) -> HttpResponse {
        let json = serde_json::to_string(self).unwrap();
        json_response(json)
    }
}

impl<T> ToListResponse for Vec<T> where T: Serialize {
    fn to_response(&self) -> HttpResponse {
        let json = serde_json::to_string(self).unwrap();
        json_response(json)
    }
}

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

