use thiserror::Error;

/// Error lexing a source file.
#[derive(Debug, Error, PartialEq, Clone, Default)]
#[doc(hidden)]
pub enum LexError {
    /// Generic lex error.
    #[default]
    #[error("parser lex error")]
    Other,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error("number expected, but got '{0}'")]
    NumberExpected(String),

    #[error("pragma declaration ($!) must be the first instruction")]
    PragmaFirst,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    #[error(transparent)]
    Rexpect(#[from] rexpect::error::Error),

    /// Error during lexing.
    #[error(transparent)]
    Lex(#[from] LexError),
}
