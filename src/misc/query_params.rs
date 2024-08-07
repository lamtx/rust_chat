use hyper::Uri;
use querystring::{querify, QueryParam};

use crate::misc::{AppResult, error};

pub struct QueryParams<'a> {
    vec: Vec<QueryParam<'a>>,
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
            .map(|(_, value)| String::from(*value))
    }

    pub fn require(&self, name: &str) -> AppResult<String> {
        match self.get(name) {
            None => error(format!("{name} is required")),
            Some(value) => Ok(value),
        }
    }

    pub fn get_list(&self, name: &str) -> Vec<String> {
        let value = self.vec.iter()
            .find(|(key, _)| key == &name);
        match value {
            None => const { Vec::new() }
            Some((_, a)) => a.split(',').map(|e| String::from(e)).collect()
        }
    }
}

pub trait Params {
    fn parse(params: &QueryParams) -> AppResult<Self>
    where
        Self: Sized;

    fn parse_uri(uri: &Uri) -> AppResult<Self>
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