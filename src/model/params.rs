use crate::misc::{Params, QueryParams, Get, TryGet, Result};

pub struct CreateParams {
    pub secret: String,
    pub post: Option<String>,
    pub post_types: Option<Vec<String>>,
}

impl Params for CreateParams {
    fn parse(params: &QueryParams) -> Result<CreateParams> {
        Ok(CreateParams {
            secret: params.try_get("secret")?,
            post: params.get("post"),
            post_types: params.get("postTypes"),
        })
    }
}

pub struct DestroyParams {
    pub secret: String,
}

pub struct JoinParams {
    pub username: Option<String>,
    pub display: Option<String>,
    pub image_url: Option<String>,
}

pub struct LastAnnouncementParams {
    pub types: Vec<String>,
}

pub struct PhotoParams {
    pub username: String,
}