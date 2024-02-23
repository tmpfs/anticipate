//! This module container a [LogStream]
//! which can wrap other streams in order to log a read/write operations.

use std::{
    io::{self, Read, Result, Write},
    ops::{Deref, DerefMut},
};

use crate::process::NonBlocking;

/// Trait for types that log output messages.
pub trait LogWriter {
    /// Log a read from the child program.
    fn log_read(&self, writer: &mut impl Write, data: &[u8]);
    /// Log a write to the child program.
    fn log_write(&self, writer: &mut impl Write, data: &[u8]);
}

/// Default log writer prefixes read and writes.
///
/// If the data can be converted to UTF-8 it is printed
/// as a string otherwise a debug representation of the
/// bytes are printed.
#[derive(Debug)]
pub struct DefaultLogWriter;

impl DefaultLogWriter {
    fn log(mut writer: impl Write, target: &str, data: &[u8]) {
        let _ = match std::str::from_utf8(data) {
            Ok(data) => writeln!(writer, "{}: {:?}", target, data),
            Err(..) => writeln!(writer, "{}:(bytes): {:?}", target, data),
        };
    }
}

impl LogWriter for DefaultLogWriter {
    fn log_read(&self, writer: &mut impl Write, data: &[u8]) {
        Self::log(writer, "read", data);
    }

    fn log_write(&self, writer: &mut impl Write, data: &[u8]) {
        Self::log(writer, "write", data);
    }
}

/// Tee log writer does not format read and write logs.
#[derive(Debug)]
pub struct TeeLogWriter;

impl LogWriter for TeeLogWriter {
    fn log_read(&self, writer: &mut impl Write, data: &[u8]) {
        let _ = writer.write_all(data);
    }

    fn log_write(&self, writer: &mut impl Write, data: &[u8]) {
        let _ = writer.write_all(data);
    }
}

/// LogStream a IO stream wrapper,
/// which logs each write/read operation.
#[derive(Debug)]
pub struct LogStream<S, W, O: LogWriter> {
    stream: S,
    logger: W,
    output: O,
}

impl<S, W, O: LogWriter> LogStream<S, W, O> {
    /// Creates a new instance of the stream.
    pub fn new(stream: S, logger: W, output: O) -> Self {
        Self {
            stream,
            logger,
            output,
        }
    }
}

impl<S, W: Write, O: LogWriter> LogStream<S, W, O> {
    fn log_write(&mut self, buf: &[u8]) {
        self.output.log_write(&mut self.logger, buf);
    }

    fn log_read(&mut self, buf: &[u8]) {
        self.output.log_read(&mut self.logger, buf);
    }
}

impl<S: Write, W: Write, O: LogWriter> Write for LogStream<S, W, O> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let n = self.stream.write(buf)?;
        self.log_write(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> Result<()> {
        self.stream.flush()
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> Result<usize> {
        let n = self.stream.write_vectored(bufs)?;

        let mut rest = n;
        let mut bytes = Vec::new();
        for buf in bufs {
            let written = std::cmp::min(buf.len(), rest);
            rest -= written;

            bytes.extend(&buf.as_ref()[..written]);

            if rest == 0 {
                break;
            }
        }

        self.log_write(&bytes);

        Ok(n)
    }
}

impl<S: Read, W: Write, O: LogWriter> Read for LogStream<S, W, O> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.stream.read(buf)?;
        self.log_read(&buf[..n]);
        Ok(n)
    }
}

impl<S: NonBlocking, W, O: LogWriter> NonBlocking for LogStream<S, W, O> {
    fn set_non_blocking(&mut self) -> Result<()> {
        self.stream.set_non_blocking()
    }

    fn set_blocking(&mut self) -> Result<()> {
        self.stream.set_blocking()
    }
}

impl<S, W, O: LogWriter> Deref for LogStream<S, W, O> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl<S, W, O: LogWriter> DerefMut for LogStream<S, W, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}
