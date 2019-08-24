
// standard imports

// third-party imports
use serde_json;

// local imports
use failure;


#[derive(Debug)]
pub struct Error {
    inner: failure::Context<ErrorKind>,
}

impl Error {
    pub fn registration_error(service: &str, error: RegistrationError) -> Error {
        return ErrorKind::ServiceRegistrationError(service.to_string(), error).into()
    }

    pub fn client_error(client: std::net::SocketAddr, error: ClientError) -> Error {
        return ErrorKind::ConnectedAppError(client, error).into()
    }
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
    #[fail(display = "io error: {}", _0)]
    IoError(#[cause] std::io::Error),
    #[fail(display = "failed to register service {} -> {:?}", _0, _1)]
    ServiceRegistrationError(String, RegistrationError),
    #[fail(display = "client {} -> {:?}", _0, _1)]
    ConnectedAppError(std::net::SocketAddr, ClientError),

    #[fail(display = "invalid rpc error: rpc {} is not registered", _0)]
    RpcError(String),
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

impl std::convert::From<std::io::Error> for Error {
    fn from(io_err: std::io::Error) -> Error {
        Error{ inner: failure::Context::new(ErrorKind::IoError(io_err)) }
    }
}


//
//
//
#[derive(Debug)]
pub struct RegistrationError {
    inner: failure::Context<RegistrationErrorKind>,
}

impl RegistrationError {
    pub fn handle_already_exists(handle: &str) -> Self {
        RegistrationErrorKind::HandleAlreadyMapped(handle.to_string()).into()
    }
}

#[derive(Debug, failure::Fail)]
pub enum RegistrationErrorKind {
    #[fail(display = "Existing handle found for endpoint {}", _0)]
    HandleAlreadyMapped(String),
}

impl std::convert::From<RegistrationErrorKind> for RegistrationError {
    fn from(kind: RegistrationErrorKind) -> RegistrationError {
        RegistrationError{ inner: failure::Context::new(kind) }
    }
}


//
//
//
#[derive(Debug)]
pub struct ClientError {
    inner: failure::Context<ClientErrorKind>,
}

impl ClientError {
    pub fn strong_references_to(handle: &str) -> Self {
        ClientErrorKind::DeregisterHandleHeld(handle.to_string()).into()
    }
}

#[derive(Debug, failure::Fail)]
pub enum ClientErrorKind {
    #[fail(display = "Multiple strong references held to {} when attempting to deregister client", _0)]
    DeregisterHandleHeld(String),
}

impl std::convert::From<ClientErrorKind> for ClientError {
    fn from(kind: ClientErrorKind) -> ClientError {
        ClientError{ inner: failure::Context::new(kind) }
    }
}
