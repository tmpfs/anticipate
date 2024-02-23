use std::error;
use std::fmt;
use std::fmt::Display;
use std::io;

use thiserror::Error;

/// Error type for the library.
#[derive(Debug, Error)]
pub enum Error {
    /// Error in command line parsing.
    #[error("failed to parse command line")]
    CommandParsing,
    /// Error in regex parsing.
    #[error("failed to parse regex")]
    RegexParsing,
    /// An timeout was reached while waiting in expect call.
    #[error("reached the timeout for an expectation")]
    ExpectTimeout,
    /// Unhandled EOF error.
    #[error("unhandled EOF")]
    Eof,
    /// Error in IO operation.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Error in conpty.
    #[cfg(windows)]
    #[error(transparent)]
    Conpty(#[from] conpty::error::Error),
}

pub(crate) fn to_io_error<E: Display>(
    message: &'static str,
) -> impl FnOnce(E) -> io::Error {
    move |e: E| {
        io::Error::new(io::ErrorKind::Other, format!("{}; {}", message, e))
    }
}
