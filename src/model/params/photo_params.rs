use crate::misc::{Params, ParseParamError, QueryParams};

pub struct PhotoParams {
    pub username: String,
}

impl Params for PhotoParams {
    fn parse<'a>(params: &QueryParams) -> Result<Self, ParseParamError<'a>> {
        Ok(PhotoParams {
            username: params.require("username")?,
        })
    }
}
