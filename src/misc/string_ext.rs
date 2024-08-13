pub trait StringExt {
    fn substring_after_last(&self, c: char) -> Self;
    fn substring_before_last(&self, c: char) -> Self;
}

impl StringExt for &str {
    fn substring_after_last(&self, c: char) -> Self {
        if let Some(index) = self.rfind(c) {
            &self[index + 1..]
        } else {
            self
        }
    }

    fn substring_before_last(&self, c: char) -> Self {
        if let Some(index) = self.rfind(c) {
            &self[..index]
        } else {
            self
        }
    }
}

pub trait OrEmpty {
    fn or_empty(&self) -> &str;
}

impl OrEmpty for Option<String> {
    fn or_empty(&self) -> &str {
        return match self {
            None => "",
            Some(value) => value,
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::misc::string_ext::StringExt;

    #[test]
    fn it_works() {
        assert_eq!("path/to/home".substring_after_last('/'), "home");
        assert_eq!("path/to/home".substring_before_last('/'), "path/to");
        assert_eq!("path/to/home".substring_after_last('?'), "path/to/home");
        assert_eq!("".substring_after_last('/'), "");
        assert_eq!("path/to/home".substring_before_last('?'), "path/to/home");
        assert_eq!("path/to/home".substring_after_last('e'), "");
        assert_eq!("path/to/home".substring_before_last('e'), "path/to/hom");
    }
}
