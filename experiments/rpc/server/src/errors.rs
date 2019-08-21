
// standard imports

// third-party imports
use serde_json;

// local imports
use failure;


#[derive(Debug)]
pub struct Error {
    inner: failure::Context<ErrorKind>,
}

impl failure::Fail for Error {
    fn cause(&self) -> Option<&dyn failure::Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&failure::Backtrace> {
        self.inner.backtrace()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}


#[derive(Debug, failure::Fail)]
pub enum ErrorKind {
    #[fail(display = "error in serialization: {}", _0)]
    SerializationError(#[cause] serde_json::error::Error),
    // ...
    // #[doc(hidden)]
    // __Nonexhaustive,
}


// Make the Error type returnable with the usage of `Ok(...?)`
impl std::convert::From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error{ inner: failure::Context::new(kind) }
    }
}

impl std::convert::From<failure::Context<ErrorKind>> for Error {
    fn from(inner: failure::Context<ErrorKind>) -> Error {
        Error{ inner: inner }
    }
}

// Using `.with_context(... SerializationError)` doesn't work because a reference
// To the error gets passed with that function and serde_json::error::Error doesn't implement Copy
impl std::convert::From<serde_json::error::Error> for Error {
    fn from(json_err: serde_json::error::Error) -> Error {
        Error{ inner: failure::Context::new(ErrorKind::SerializationError(json_err)) }
    }
}
