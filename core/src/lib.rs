mod error;
mod interpreter;
mod parser;

pub use error::Error;
pub use interpreter::{CompileOptions, ScriptFile};
pub use parser::*;

/// Result type for the parser.
pub type Result<T> = std::result::Result<T, Error>;
