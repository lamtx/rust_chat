use crate::misc::{AppResult, Params, QueryParams};

pub struct CreateParams {
    pub secret: String,
    pub post: Option<String>,
    pub post_types: Vec<String>,
}

impl Params for CreateParams {
    fn parse(params: &QueryParams) -> AppResult<CreateParams> {
        Ok(CreateParams {
            secret: params.require("secret")?,
            post: params.get("post"),
            post_types: params.get_list("postTypes"),
        })
    }
}