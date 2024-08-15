use std::sync::Arc;

use hyper::{Request, StatusCode};
use hyper_tls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::Serialize;

use crate::log;

pub struct RestClient {
    post_url: String,
    client: Arc<Client<HttpsConnector<HttpConnector>, String>>,
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
                Ok(response) => {
                    // let mut buf = Vec::with_capacity(1024).writer();
                    // while let Some(next) = response.frame().await {
                    //     if let Ok(frame) = next {
                    //         if let Some(chunk) = frame.data_ref() {
                    //             buf.write_all(&chunk).unwrap();
                    //         }
                    //     } else {
                    //         break;
                    //     }
                    // }
                    // let body = String::from_utf8(buf.into_inner()).unwrap();
                    if response.status() == StatusCode::OK {
                        log!("posted ok")
                    } else {
                        eprintln!("posted failed: {}", response.status());
                    }
                }
                Err(e) => {
                    eprintln!("posted error: {:?}", e);
                }
            }
        });
    }
}
