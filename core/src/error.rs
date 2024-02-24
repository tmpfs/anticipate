use std::time::Duration;
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
    #[error("reached the timeout of {0:?} expecting {1}")]
    ExpectTimeout(Duration, String),
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
