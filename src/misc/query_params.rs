use hyper::Uri;
use querystring::{querify, QueryParam};
use urlencoding::decode;

pub struct QueryParams<'a> {
    vec: Vec<QueryParam<'a>>,
}

pub enum ParseParamError<'a> {
    FieldRequired { name: &'a str }
}

impl<'a> QueryParams<'a> {
    pub fn parse(s: Option<&'a str>) -> QueryParams<'a> {
        match s {
            None => QueryParams { vec: const { Vec::new() } },
            Some(s) => QueryParams { vec: querify(s) },
        }
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.vec.iter()
            .find(|(key, _)| key == &name)
            .map(|(_, value)| decode_url(value))
    }

    pub fn require<'b>(&self, name: &'b str) -> Result<String, ParseParamError<'b>> {
        match self.get(name) {
            None => Err(ParseParamError::FieldRequired { name }),
            Some(value) => Ok(value),
        }
    }

    pub fn get_list(&self, name: &str) -> Vec<String> {
        let value = self.vec.iter()
            .find(|(key, _)| key == &name);
        match value {
            None => const { Vec::new() }
            Some((_, a)) => a.split(',').map(|e| decode_url(e)).collect()
        }
    }
}

#[inline]
fn decode_url(value: &str) -> String {
    decode(value).unwrap().into_owned()
}

pub trait Params {
    fn parse<'a>(params: &QueryParams) -> Result<Self, ParseParamError<'a>>
    where
        Self: Sized;

    fn parse_uri<'a>(uri: &Uri) -> Result<Self, ParseParamError<'a>>
    where
        Self: Sized,
    {
        Self::parse(&QueryParams::parse(uri.query()))
    }
}

pub trait GetQueryFromUri {
    fn query_params(&self) -> QueryParams;
}

impl GetQueryFromUri for Uri {
    fn query_params(&self) -> QueryParams {
        QueryParams::parse(self.query())
    }
}