use crate::misc::{Params, QueryParams, Get, TryGet, AppResult};

pub struct CreateParams {
    pub secret: String,
    pub post: Option<String>,
    pub post_types: Option<Vec<String>>,
}

impl Params for CreateParams {
    fn parse(params: &QueryParams) -> AppResult<CreateParams> {
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

impl Params for DestroyParams {
    fn parse(params: &QueryParams) -> AppResult<DestroyParams> {
        Ok(DestroyParams {
            secret: params.try_get("secret")?,
        })
    }
}

pub struct JoinParams {
    pub username: Option<String>,
    pub display: Option<String>,
    pub image_url: Option<String>,
}

impl Params for JoinParams {
    fn parse(params: &QueryParams) -> AppResult<JoinParams> {
        Ok(JoinParams {
            username: params.get("username"),
            display: params.get("display"),
            image_url: params.get("imageUrl"),
        })
    }
}

pub struct LastAnnouncementParams {
    pub types: Vec<String>,
}

pub struct PhotoParams {
    pub username: String,
}