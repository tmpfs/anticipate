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

mod log;
mod session;

use crate::{process::Process, Error};
use std::{process::Command, time::Duration};

#[cfg(unix)]
type OsProc = crate::process::unix::UnixProcess;
#[cfg(windows)]
type OsProc = crate::process::windows::WinProcess;

#[cfg(unix)]
type OsProcStream = crate::process::unix::PtyStream;
#[cfg(windows)]
type OsProcStream = crate::process::windows::ProcessStream;

/// OS process which can run a [`Session`] and a default one.
pub type OsProcess = OsProc;
/// OS process stream which is a default one for [`Session`].
pub type OsProcessStream = OsProcStream;

pub use log::*;
pub use session::Session;

pub type DefaultSession = Session<DefaultLogWriter>;

/// Spawn a session with logger and timeout options.
pub fn spawn_with_options<O: LogWriter>(
    command: Command,
    logger: Option<O>,
    timeout: Option<Duration>,
) -> Result<Session<O>, Error> {
    let mut process = OsProcess::spawn_command(command)?;
    let stream = process.open_stream()?;
    Ok(Session::<O>::new(process, stream, logger, timeout)?)
}

impl<O: LogWriter> Session<O> {
    /// Spawns a session from a string command.
    pub(crate) fn spawn_cmd(
        cmd: &str,
        timeout: Option<Duration>,
    ) -> Result<Self, Error> {
        let mut process = OsProcess::spawn(cmd)?;
        let stream = process.open_stream()?;
        Ok(Self::new(process, stream, None, timeout)?)
    }
}

impl Session<DefaultLogWriter> {
    /// Spawns a session on a platform process.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::process::Command;
    /// use anticipate::{Session, DefaultLogWriter};
    ///
    /// let p = Session::<DefaultLogWriter>::spawn(Command::new("cat"));
    /// ```
    pub fn spawn(command: Command) -> Result<Self, Error> {
        let mut process = OsProcess::spawn_command(command)?;
        let stream = process.open_stream()?;
        Ok(Self::new(process, stream, Some(DefaultLogWriter), None)?)
    }
}

impl Session<PrefixLogWriter> {
    /// Spawns a session on a platform process.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::process::Command;
    /// use anticipate::{Session, PrefixLogWriter};
    ///
    /// let p = Session::<PrefixLogWriter>::spawn(Command::new("cat"));
    /// ```
    pub fn spawn(command: Command) -> Result<Self, Error> {
        let mut process = OsProcess::spawn_command(command)?;
        let stream = process.open_stream()?;
        Ok(Self::new(process, stream, Some(PrefixLogWriter), None)?)
    }
}

impl Session<TeeLogWriter> {
    /// Spawns a session on a platform process.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::process::Command;
    /// use anticipate::{Session, TeeLogWriter};
    ///
    /// let p = Session::<TeeLogWriter>::spawn(Command::new("cat"));
    /// ```
    pub fn spawn(command: Command) -> Result<Self, Error> {
        let mut process = OsProcess::spawn_command(command)?;
        let stream = process.open_stream()?;
        Ok(Self::new(process, stream, Some(TeeLogWriter), None)?)
    }
}
