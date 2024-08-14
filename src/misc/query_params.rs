use hyper::Uri;
use querystring::{querify, QueryParam};
use urlencoding::decode;

pub struct QueryParams<'a> {
    vec: Vec<QueryParam<'a>>,
}

#[derive(Debug)]
pub enum ParseParamError<'a> {
    FieldRequired { name: &'a str },
}

macro_rules! empty_vec {
    () => {
        const { Vec::new() }
    };
}

impl<'a> QueryParams<'a> {
    pub fn parse(query: Option<&'a str>) -> QueryParams<'a> {
        match query {
            None => QueryParams { vec: empty_vec!() },
            Some(s) => QueryParams { vec: querify(s) },
        }
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.vec
            .iter()
            .find(|(key, _)| key == &name)
            .and_then(|(_, value)| decode(value).ok())
            .map(|e| e.into_owned())
    }

    pub fn require<'b>(&self, name: &'b str) -> Result<String, ParseParamError<'b>> {
        match self.get(name) {
            None => Err(ParseParamError::FieldRequired { name }),
            Some(value) => Ok(value),
        }
    }

    pub fn get_list(&self, name: &str) -> Vec<String> {
        self.vec
            .iter()
            .find(|(key, _)| key == &name)
            .and_then(|(_, a)| decode(a).ok())
            .map_or_else(
                || empty_vec!(),
                |s| s.split(',').map(|e| e.to_owned()).collect(),
            )
    }
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
