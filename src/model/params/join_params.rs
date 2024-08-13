use crate::misc::{Params, ParseParamError, QueryParams};

#[derive(Debug)]
pub struct JoinParams {
    pub username: Option<String>,
    pub display: Option<String>,
    pub image_url: Option<String>,
}

impl Params for JoinParams {
    fn parse<'a>(params: &QueryParams) -> Result<JoinParams, ParseParamError<'a>> {
        Ok(JoinParams {
            username: params.get("username"),
            display: params.get("display"),
            image_url: params.get("imageUrl"),
        })
    }
}

#[cfg(test)]
mod tests {
    use hyper::Uri;

    use crate::misc::Params;
    use crate::model::JoinParams;

    #[test]
    fn it_works() {
        let url = "ws://10.0.2.2:9339/dev/528/join?username=133&display=TH%E1%BB%AC%20NGHI%E1%BB%86M&imageUrl=https%3A%2F%2Fdev.shoplive.vn%2Fcontent%2Fimages%2Favatars%2F133.jpg";
        let uri = url.parse::<Uri>().unwrap();
        let params = JoinParams::parse_uri(&uri).unwrap();
        println!("{:?}", params);
        assert_eq!(params.display, Some("THỬ NGHIỆM".to_string()));
        assert_eq!(
            params.image_url,
            Some("https://dev.shoplive.vn/content/images/avatars/133.jpg".to_string())
        );
    }
}
