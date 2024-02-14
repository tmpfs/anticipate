mod compiler;
mod error;
mod parser;

pub use compiler::{CompileOptions, ScriptFile};
pub use error::Error;
pub use parser::*;

/// Result type for the parser.
pub type Result<T> = std::result::Result<T, Error>;
