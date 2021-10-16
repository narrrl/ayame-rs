use serenity::Error as SerenityError;
use std::{
    borrow::Cow,
    error::Error as StdError,
    fmt::{self, Display, Formatter},
};

//
// heavily *inspired* by
// [serenity_utils](https://github.com/AriusX7/serenity-utils/blob/current/src/error.rs)

#[derive(Debug)]
pub enum Error {
    SerenityError(SerenityError),
    TimeoutError,
    InvalidInput,
    Other(String),
}

impl StdError for Error {}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let err = match self {
            Error::SerenityError(e) => Cow::from(e.to_string()),
            Error::TimeoutError => Cow::from("timout occured"),
            Error::InvalidInput => Cow::from("invalid input"),
            Error::Other(e) => Cow::from(e),
        };

        write!(f, "{}", err)
    }
}

impl<'a> From<&'a str> for Error {
    fn from(error: &'a str) -> Self {
        Self::Other(error.to_string())
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Self::Other(error)
    }
}

impl From<SerenityError> for Error {
    fn from(error: SerenityError) -> Self {
        Self::SerenityError(error)
    }
}
