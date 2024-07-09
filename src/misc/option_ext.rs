pub trait OptionExt<T: PartialEq> {
    fn contains(&self, value: &T) -> bool;
}

impl<T: PartialEq> OptionExt<T> for Option<T> {
    fn contains(&self, value: &T) -> bool {
        match self {
            None => false,
            Some(unwrapped) => unwrapped == value,
        }
    }
}