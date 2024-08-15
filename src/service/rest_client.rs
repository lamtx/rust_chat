use bytes::BufMut;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use hyper_tls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::Serialize;
use std::io::Write;
use std::sync::Arc;

use crate::log;

pub struct RestClient {
    post_url: String,
    client: Arc<Client<HttpsConnector<HttpConnector>, String>>,
}
#[cfg(debug_assertions)]
async fn read_body(response: &mut Response<Incoming>) -> String {
    let mut buf = Vec::with_capacity(1024).writer();
    while let Some(next) = response.frame().await {
        if let Ok(frame) = next {
            if let Some(chunk) = frame.data_ref() {
                buf.write_all(&chunk).unwrap();
            }
        } else {
            break;
        }
    }
    String::from_utf8(buf.into_inner()).unwrap_or_default()
}
#[cfg(not(debug_assertions))]
async fn read_body(response: &mut Response<Incoming>) -> String {
    String::default()
}

impl RestClient {
    pub fn create(post_url: String) -> RestClient {
        RestClient {
            post_url,
            client: Arc::new(Client::builder(TokioExecutor::new()).build(HttpsConnector::new())),
        }
    }

    pub fn spawn_post<M>(&self, message: &M)
    where
        M: Serialize,
    {
        let request = Request::builder()
            .uri(&self.post_url)
            .method(hyper::Method::POST)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_string(message).unwrap())
            .unwrap();
        let client = self.client.clone();
        tokio::spawn(async move {
            match client.request(request).await {
                Ok(mut response) => {
                    let body = read_body(&mut response).await;
                    if response.status() == StatusCode::OK {
                        log!("posted ok:{body}")
                    } else {
                        eprintln!("posted failed: {}-{body}", response.status());
                    }
                }
                Err(e) => {
                    eprintln!("posted error: {:?}", e);
                }
            }
        });
    }
}
