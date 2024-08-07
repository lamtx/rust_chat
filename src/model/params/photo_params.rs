use crate::misc::{AppResult, Params, QueryParams};

pub struct PhotoParams {
    pub username: String,
}

impl Params for PhotoParams {
    fn parse(params: &QueryParams) -> AppResult<PhotoParams> {
        Ok(PhotoParams {
            username: params.require("username")?
        })
    }
}
