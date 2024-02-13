use thiserror::Error;

/// Error lexing the keychain dump.
#[derive(Debug, Error, PartialEq, Clone, Default)]
#[doc(hidden)]
pub enum LexError {
    /// Generic lex error.
    #[default]
    #[error("keychain parser lex")]
    Other,
}

#[derive(Debug, Error)]
pub enum Error {
    /// Error during lexing.
    #[error(transparent)]
    Lex(#[from] LexError),
}
