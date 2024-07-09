use futures::future::err;
use hyper::Uri;
use querystring::{querify, QueryParams as P};
use crate::misc::{error, AppResult};

pub trait Get<T> {
    fn get(&self, name: &str) -> T;
}

pub trait TryGet<T> {
    fn try_get(&self, name: &str) -> AppResult<T>;
}

pub struct QueryParams<'a> (pub P<'a>);

impl<'a> QueryParams<'a> {
    pub fn parse(s: Option<&'a str>) -> QueryParams<'a> {
        const EMPTY_VEC: P = Vec::new();
        match s {
            None => QueryParams(EMPTY_VEC),
            Some(s) => QueryParams(querify(s)),
        }
    }
}

pub fn parse_param<T>(uri: &Uri) -> AppResult<T> where T: Params {
    T::parse(&QueryParams::parse(uri.query()))
}

pub trait Params {
    fn parse(params: &QueryParams) -> AppResult<Self>
        where Self: Sized;

    fn parse_uri(uri: &Uri) -> AppResult<Self> where Self: Sized {
        Self::parse(&QueryParams::parse(uri.query()))
    }
}

impl<'a> Get<Option<String>> for QueryParams<'a> {
    fn get(&self, name: &str) -> Option<String> {
        self.0.iter()
            .find(|(key, _)| key == &name)
            .map(|(_, value)| String::from(*value))
    }
}

impl<'a> TryGet<String> for QueryParams<'a> {
    fn try_get(&self, name: &str) -> AppResult<String> {
        let value: Option<String> = self.get(name);
        match value {
            None => error(format!("{name} is required")),
            Some(value) => Ok(value),
        }
    }
}

impl<'a> Get<Option<Vec<String>>> for QueryParams<'a> {
    fn get(&self, name: &str) -> Option<Vec<String>> {
        self.0.iter()
            .find(|(key, _)| key == &name)
            .map(|(_, value)| {
                value.split(',').map(|e| String::from(e)).collect()
            })
    }
}