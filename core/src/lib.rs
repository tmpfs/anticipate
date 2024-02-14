//! Parser and interpreter for the anticipate script automation tool.
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
