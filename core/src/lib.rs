#![warn(missing_docs)]
#![forbid(unsafe_code)]
//! Control a pseudo-terminal similar to `expect(1)`,
//! fork of [expectrl](https://docs.rs/expectrl) with
//! minimal dependencies and features.

mod captures;
mod control_code;
mod error;
mod needle;

pub mod log;
pub mod process;
pub mod repl;
pub(crate) mod session;

pub use captures::Captures;
pub use control_code::ControlCode;
pub use error::Error;
pub use needle::{Any, Eof, NBytes, Needle, Regex};

#[cfg(unix)]
pub use ptyprocess::{Signal, WaitStatus};

pub use session::*;

use std::io::{BufRead, Read, Write};

/// Spawn a command.
pub fn spawn<S: AsRef<str>>(cmd: S) -> Result<DefaultSession, Error> {
    DefaultSession::spawn_cmd(cmd.as_ref(), None)
}

/// Trait for types that can read and write to child programs.
pub trait Expect: Write + Read + BufRead {
    /// Send a buffer to the child program.
    fn send<B: AsRef<[u8]>>(&mut self, buf: B) -> std::io::Result<()>;

    /// Send a line to the child program.
    fn send_line(&mut self, text: &str) -> std::io::Result<()>;

    /// Expect output from the child program.
    fn expect<N>(&mut self, needle: N) -> Result<Captures, Error>
    where
        N: Needle;
}
