use crate::misc::{AppResult, Params, QueryParams};

pub struct DestroyParams {
    pub secret: String,
}

impl Params for DestroyParams {
    fn parse(params: &QueryParams) -> AppResult<DestroyParams> {
        Ok(DestroyParams {
            secret: params.require("secret")?,
        })
    }
}
