use crate::misc::{Params, ParseParamError, QueryParams};

#[derive(Debug)]
pub struct LastAnnouncementParams {
    pub types: Vec<String>,
}

impl Params for LastAnnouncementParams {
    fn parse<'a>(params: &QueryParams) -> Result<Self, ParseParamError<'a>> {
        Ok(LastAnnouncementParams {
            types: params.get_list("types"),
        })
    }
}
