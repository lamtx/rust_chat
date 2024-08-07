use crate::misc::{AppResult, Params, QueryParams};

pub struct LastAnnouncementParams {
    pub types: Vec<String>,
}

impl Params for LastAnnouncementParams {
    fn parse(params: &QueryParams) -> AppResult<LastAnnouncementParams> {
        Ok(LastAnnouncementParams {
            types: params.get_list("types")
        })
    }
}
