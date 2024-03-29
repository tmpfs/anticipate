//! This module contains a Windows implementation of [crate::process::Process].

use std::{
    io::{self, Read, Result, Write},
    ops::{Deref, DerefMut},
    process::Command,
};

use conpty::{
    io::{PipeReader, PipeWriter},
    spawn, Process,
};

use super::{Healthcheck, NonBlocking, Process as ProcessTrait};

/// A windows representation of a [Process] via [conpty::Process].
#[derive(Debug)]
pub struct WinProcess {
    proc: Process,
}

impl ProcessTrait for WinProcess {
    type Command = Command;
    type Stream = ProcessStream;

    fn spawn<S: AsRef<str>>(cmd: S) -> Result<Self> {
        Ok(spawn(cmd.as_ref()).map(|proc| WinProcess { proc })?)
    }

    fn spawn_command(command: Self::Command) -> Result<Self> {
        Ok(
            conpty::Process::spawn(command)
                .map(|proc| WinProcess { proc })?,
        )
    }

    fn open_stream(&mut self) -> Result<Self::Stream> {
        let input = self.proc.input()?;
        let output = self.proc.output()?;
        Ok(Self::Stream::new(output, input))
    }
}

impl Healthcheck for WinProcess {
    fn is_alive(&mut self) -> Result<bool> {
        Ok(self.proc.is_alive())
    }
}

impl Deref for WinProcess {
    type Target = Process;

    fn deref(&self) -> &Self::Target {
        &self.proc
    }
}

impl DerefMut for WinProcess {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.proc
    }
}

/// An IO stream of [WinProcess].
#[derive(Debug)]
pub struct ProcessStream {
    input: PipeWriter,
    output: PipeReader,
}

impl ProcessStream {
    fn new(output: PipeReader, input: PipeWriter) -> Self {
        Self { input, output }
    }

    /// Tries to clone the stream.
    pub fn try_clone(
        &self,
    ) -> std::result::Result<Self, conpty::error::Error> {
        Ok(Self {
            input: self.input.try_clone()?,
            output: self.output.try_clone()?,
        })
    }
}

impl Write for ProcessStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.input.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.input.flush()
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> Result<usize> {
        self.input.write_vectored(bufs)
    }
}

impl Read for ProcessStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.output.read(buf)
    }
}

impl NonBlocking for ProcessStream {
    fn set_non_blocking(&mut self) -> Result<()> {
        self.output.blocking(false);
        Ok(())
    }

    fn set_blocking(&mut self) -> Result<()> {
        self.output.blocking(true);
        Ok(())
    }
}
