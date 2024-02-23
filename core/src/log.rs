//! Types for writing and formatting logs to stdout.
use std::io::Write;

/// Trait for types that log read and writes to a child program.
pub trait LogWriter {
    /// Log a read from the child program.
    fn log_read(&self, writer: &mut impl Write, data: &[u8]);
    /// Log a write to the child program.
    fn log_write(&self, writer: &mut impl Write, data: &[u8]);
}

/// Noop log writer does not log anything.
#[derive(Debug)]
pub struct NoopLogWriter;

impl LogWriter for NoopLogWriter {
    fn log_read(&self, _writer: &mut impl Write, _data: &[u8]) {}
    fn log_write(&self, _writer: &mut impl Write, _data: &[u8]) {}
}

/// Prefix log writer prefixes read and writes.
///
/// If the data can be converted to UTF-8 it is printed
/// as a string otherwise a debug representation of the
/// bytes are printed.
///
/// Be aware that if you are writing data that would be masked,
/// for example, entering a password at an interactive prompt
/// the plain text value will be logged.
#[derive(Debug)]
pub struct PrefixLogWriter;

impl PrefixLogWriter {
    fn log(mut writer: impl Write, target: &str, data: &[u8]) {
        let _ = match std::str::from_utf8(data) {
            Ok(data) => writeln!(writer, "{}: {:?}", target, data),
            Err(..) => writeln!(writer, "{}:(bytes): {:?}", target, data),
        };
    }
}

impl LogWriter for PrefixLogWriter {
    fn log_read(&self, writer: &mut impl Write, data: &[u8]) {
        Self::log(writer, "read", data);
    }

    fn log_write(&self, writer: &mut impl Write, data: &[u8]) {
        Self::log(writer, "write", data);
    }
}

/// Standard log writer does not format read and write logs.
///
/// Be aware that if you are writing data that would be masked,
/// for example, entering a password at an interactive prompt
/// the plain text value will be logged.
#[derive(Debug)]
pub struct StandardLogWriter;

impl LogWriter for StandardLogWriter {
    fn log_read(&self, writer: &mut impl Write, data: &[u8]) {
        let _ = writer.write_all(data);
    }

    fn log_write(&self, writer: &mut impl Write, data: &[u8]) {
        let _ = writer.write_all(data);
    }
}
