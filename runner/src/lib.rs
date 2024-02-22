//! Parser and interpreter for the anticipate script automation tool.
//!
//! Moved to [anticipate-runner]().
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod error;
mod interpreter;
mod parser;

pub use error::Error;
pub use interpreter::{CinemaOptions, InterpreterOptions, ScriptFile};
pub use parser::*;

/// Result type for the parser.
pub type Result<T> = std::result::Result<T, Error>;

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

/// Resolve a possibly relative path.
pub(crate) fn resolve_path(
    base: impl AsRef<Path>,
    input: &str,
) -> Result<Cow<str>> {
    let path = PathBuf::from(input);
    if path.is_relative() {
        if let Some(parent) = base.as_ref().parent() {
            let new_path = parent.join(input);
            let path = new_path.canonicalize()?;
            Ok(Cow::Owned(path.to_string_lossy().as_ref().to_owned()))
        } else {
            Ok(Cow::Borrowed(input))
        }
    } else {
        Ok(Cow::Borrowed(input))
    }
}
