use serde::Deserialize;

use crate::misc::{Params, ParseParamError, QueryParams};

#[derive(Deserialize)]
pub struct CreateParams {
    pub secret: String,
    pub post: Option<String>,
    #[serde(default)]
    pub post_types: Vec<String>,
}

impl Params for CreateParams {
    fn parse<'a>(params: &QueryParams) -> Result<Self, ParseParamError<'a>> {
        Ok(CreateParams {
            secret: params.require("secret")?,
            post: params.get("post"),
            post_types: params.get_list("postTypes"),
        })
    }
}
