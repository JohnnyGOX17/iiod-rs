use std::fmt;

/// Errors returned by iiod-rs operations.
#[derive(Debug)]
pub enum Error {
    /// An I/O error from the underlying transport.
    Io(std::io::Error),
    /// The remote `iiod` returned a negative errno code.
    IiodError {
        code: i32,
        msg: String,
    },
    /// Failed to parse a response from the daemon.
    Protocol(String),
    /// Failed to parse XML from a PRINT response.
    Xml(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::IiodError { code, msg } => write!(f, "iiod error {code}: {msg}"),
            Error::Protocol(msg) => write!(f, "protocol error: {msg}"),
            Error::Xml(msg) => write!(f, "XML parse error: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
