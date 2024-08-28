pub trait OptionExt<T: PartialEq> {
    fn eq_to_some(&self, value: &T) -> bool;
}

impl<T: PartialEq> OptionExt<T> for Option<T> {
    fn eq_to_some(&self, value: &T) -> bool {
        match self {
            None => false,
            Some(unwrapped) => unwrapped == value,
        }
    }
}
