use serde::Deserialize;

use crate::misc::{Params, ParseParamError, QueryParams};

#[derive(Deserialize)]
pub struct DestroyParams {
    pub secret: String,
}

impl Params for DestroyParams {
    fn parse<'a>(params: &QueryParams) -> Result<Self, ParseParamError<'a>> {
        Ok(DestroyParams {
            secret: params.require("secret")?,
        })
    }
}
