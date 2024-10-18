use std::fmt::{Debug, Display};

pub trait PrintErrorSimple {
    fn print_error(&self, prefix: &str);
}

#[allow(unused)]
pub trait PrintErrorDetails {
    fn print_error_detail(&self, prefix: &str);
}

impl<T, E: Display> PrintErrorSimple for Result<T, E> {
    fn print_error(&self, prefix: &str) {
        if let Err(e) = self {
            log::error!("{prefix} error: {e}")
        }
    }
}

impl<T, E: Debug> PrintErrorDetails for Result<T, E> {
    fn print_error_detail(&self, prefix: &str) {
        if let Err(e) = self {
            log::error!("{prefix} error: {e:?}")
        }
    }
}
