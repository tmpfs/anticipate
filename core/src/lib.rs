#![warn(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Control a pseudo-terminal similar to `expect(1)`.
//!
//! Fork of [expectrl](https://docs.rs/expectrl) with minimal
//! dependencies and features.
//!
//! Using the library you can:
//!
//! - Spawn process
//! - Control process
//!
//! ## Examples
//!
//! ### An example for interacting via ftp.
//!
//! ```no_run,ignore
//! use anticipate::{spawn, Regex, Eof, WaitStatus};
//!
//! let mut p = spawn("ftp speedtest.tele2.net").unwrap();
//! p.expect(Regex("Name \\(.*\\):")).unwrap();
//! p.send_line("anonymous").unwrap();
//! p.expect("Password").unwrap();
//! p.send_line("test").unwrap();
//! p.expect("ftp>").unwrap();
//! p.send_line("cd upload").unwrap();
//! p.expect("successfully changed.\r\nftp>").unwrap();
//! p.send_line("pwd").unwrap();
//! p.expect(Regex("[0-9]+ \"/upload\"")).unwrap();
//! p.send_line("exit").unwrap();
//! p.expect(Eof).unwrap();
//! assert_eq!(p.wait().unwrap(), WaitStatus::Exited(p.pid(), 0));
//! ```
//!
//! *The example inspired by the one in [philippkeller/rexpect].*
//!
//! ### An example when `Command` is used.
//!
//! ```no_run,ignore
//! use std::{process::Command, io::prelude::*};
//! use anticipate::Session;
//!
//! let mut echo_hello = Command::new("sh");
//! echo_hello.arg("-c").arg("echo hello");
//!
//! let mut p = Session::spawn(echo_hello).unwrap();
//! p.expect("hello").unwrap();
//! ```
//!
//! ### An example of logging.
//!
//! ```no_run,ignore
//! use std::io::{stdout, prelude::*};
//! use anticipate::{spawn, session::log};
//!
//! let mut sh = log(spawn("sh").unwrap(), stdout()).unwrap();
//!
//! writeln!(sh, "Hello World").unwrap();
//! ```

mod captures;
mod control_code;
mod error;
mod needle;

pub mod process;
pub mod repl;
pub mod session;
pub mod stream;

pub use captures::Captures;
pub use control_code::ControlCode;
pub use error::Error;
pub use needle::{Any, Eof, NBytes, Needle, Regex};

#[cfg(unix)]
pub use ptyprocess::{Signal, WaitStatus};

pub use session::Session;

/// Spawn spawnes a new session.
///
/// It accepts a command and possibly arguments just as string.
/// It doesn't parses ENV variables. For complex constrictions use [`Session::spawn`].
///
/// # Example
///
/// ```no_run,ignore
/// use std::{thread, time::Duration, io::{Read, Write}};
/// use anticipate::{spawn, ControlCode};
///
/// let mut p = spawn("cat").unwrap();
/// p.send_line("Hello World").unwrap();
///
/// thread::sleep(Duration::from_millis(300)); // give 'cat' some time to set up
/// p.send(ControlCode::EndOfText).unwrap(); // abort: SIGINT
///
/// let mut buf = String::new();
/// p.read_to_string(&mut buf).unwrap();
///
/// assert_eq!(buf, "Hello World\r\n");
/// ```
///
/// [`Session::spawn`]: ./struct.Session.html?#spawn
pub fn spawn<S: AsRef<str>>(cmd: S) -> Result<Session, Error> {
    Session::spawn_cmd(cmd.as_ref())
}

use std::io::{BufRead, Read, Write};

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
