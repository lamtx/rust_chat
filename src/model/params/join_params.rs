use crate::misc::{AppResult, Params, QueryParams};

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
