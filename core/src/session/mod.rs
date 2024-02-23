//! This module contains a system independent [Session] representation.
//!
//! But it does set a default [Session<P, S>] processes and stream in order to be able to use Session without generics.
//! It also sets a list of other methods which are available for a platform processes.
//!
//! # Example
//!
//! ```no_run,ignore
//! use std::{process::Command, io::prelude::*};
//! use anticipate::Session;
//!
//! let mut p = Session::spawn(Command::new("cat")).unwrap();
//! writeln!(p, "Hello World").unwrap();
//! let mut line = String::new();
//! p.read_line(&mut line).unwrap();
//! ```

#[cfg(feature = "async")]
mod async_session;
#[doc(hidden)]
pub mod pty_session;
#[cfg(not(feature = "async"))]
mod sync_session;

use std::{io::Write, process::Command};

use crate::{
    process::Process,
    stream::log::{DefaultLogWriter, LogStream, TeeLogWriter},
    Error,
};

#[cfg(not(feature = "async"))]
use std::io::Read;

#[cfg(feature = "async")]
use crate::process::IntoAsyncStream;

#[cfg(unix)]
type OsProc = crate::process::unix::UnixProcess;
#[cfg(windows)]
type OsProc = crate::process::windows::WinProcess;

#[cfg(all(unix, not(feature = "async")))]
type OsProcStream = crate::process::unix::PtyStream;
#[cfg(all(unix, feature = "async"))]
type OsProcStream = crate::process::unix::AsyncPtyStream;
#[cfg(all(windows, not(feature = "async")))]
type OsProcStream = crate::process::windows::ProcessStream;
#[cfg(all(windows, feature = "async"))]
type OsProcStream = crate::process::windows::AsyncProcessStream;

/// Session that is logged using the default writer.
pub type DefaultLogSession =
    Session<OsProc, LogStream<OsProcStream, std::io::Stdout, DefaultLogWriter>>;

/// Session that is logged using the tee writer.
pub type TeeLogSession = Session<OsProc, LogStream<OsProcStream, std::io::Stdout, TeeLogWriter>>;

/// A type alias for OS process which can run a [`Session`] and a default one.
pub type OsProcess = OsProc;
/// A type alias for OS process stream which is a default one for [`Session`].
pub type OsProcessStream = OsProcStream;

#[cfg(feature = "async")]
pub use async_session::Session;

#[cfg(not(feature = "async"))]
pub use sync_session::Session;

impl Session {
    /// Spawns a session on a platform process.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::process::Command;
    /// use anticipate::Session;
    ///
    /// let p = Session::spawn(Command::new("cat"));
    /// ```
    pub fn spawn(command: Command) -> Result<Self, Error> {
        let mut process = OsProcess::spawn_command(command)?;
        let stream = process.open_stream()?;

        #[cfg(feature = "async")]
        let stream = stream.into_async_stream()?;

        let session = Self::new(process, stream)?;

        Ok(session)
    }

    /// Spawns a session on a platform process.
    /// Using a string commandline.
    pub(crate) fn spawn_cmd(cmd: &str) -> Result<Self, Error> {
        let mut process = OsProcess::spawn(cmd)?;
        let stream = process.open_stream()?;

        #[cfg(feature = "async")]
        let stream = stream.into_async_stream()?;

        let session = Self::new(process, stream)?;

        Ok(session)
    }
}

/// Set a logger which formats and prefixes the IO.
///
/// Be aware that if you are writing data that would be masked,
/// for example, entering a password at an interactive prompt
/// the plain text value will be logged.
///
/// # Example
///
/// ```
/// use anticipate::{spawn, session::log};
///
/// let p = spawn("cat").unwrap();
/// let p = log(p, std::io::stdout());
/// ```
#[cfg(not(feature = "async"))]
pub fn log<W, P, S>(
    session: Session<P, S>,
    dst: W,
) -> Result<Session<P, LogStream<S, W, DefaultLogWriter>>, Error>
where
    W: Write,
    S: Read,
{
    session.swap_stream(|s| LogStream::new(s, dst, DefaultLogWriter))
}

/// Set a logger which formats and prefixes the IO.
///
/// Be aware that if you are writing data that would be masked,
/// for example, entering a password at an interactive prompt
/// the plain text value will be logged.
///
/// # Example
///
/// ```
/// use anticipate::{spawn, session::log};
///
/// let p = spawn("cat").unwrap();
/// let p = log(p, std::io::stdout());
/// ```
#[cfg(feature = "async")]
pub fn log<W, P, S>(
    session: Session<P, S>,
    dst: W,
) -> Result<Session<P, LogStream<S, W, DefaultLogWriter>>, Error>
where
    W: Write,
{
    session.swap_stream(|s| LogStream::new(s, dst, DefaultLogWriter))
}

/// Set a logger which does not format the IO.
///
/// Be aware that if you are writing data that would be masked,
/// for example, entering a password at an interactive prompt
/// the plain text value will be logged.
#[cfg(not(feature = "async"))]
pub fn tee<W, P, S>(
    session: Session<P, S>,
    dst: W,
) -> Result<Session<P, LogStream<S, W, TeeLogWriter>>, Error>
where
    W: Write,
    S: Read,
{
    session.swap_stream(|s| LogStream::new(s, dst, TeeLogWriter))
}

/// Set a logger which does not format the IO.
///
/// Be aware that if you are writing data that would be masked,
/// for example, entering a password at an interactive prompt
/// the plain text value will be logged.
#[cfg(feature = "async")]
pub fn log<W, P, S>(
    session: Session<P, S>,
    dst: W,
) -> Result<Session<P, LogStream<S, W, TeeLogWriter>>, Error>
where
    W: Write,
{
    session.swap_stream(|s| LogStream::new(s, dst, TeeLogWriter))
}
