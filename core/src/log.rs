//! Types for writing and formatting logs to stdout.
use std::io::Write;

/// Trait for types that log read and writes to a child program.
pub trait LogWriter {
    /// Log a read from the child program.
    fn log_read(&mut self, data: &[u8]);
    /// Log a write to the child program.
    fn log_write(&mut self, data: &[u8]);
}

/// Noop log writer does not log anything.
pub struct NoopLogWriter;

impl LogWriter for NoopLogWriter {
    fn log_read(&mut self, _data: &[u8]) {}
    fn log_write(&mut self, _data: &[u8]) {}
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
pub struct PrefixLogWriter {
    writer: Box<dyn Write>,
}

impl Default for PrefixLogWriter {
    fn default() -> Self {
        Self {
            writer: Box::new(std::io::stdout()),
        }
    }
}

impl PrefixLogWriter {
    /// Create a new prefixed log writer.     
    pub fn new(writer: Box<dyn Write>) -> Self {
        Self { writer }
    }

    fn log(&mut self, target: &str, data: &[u8]) {
        let _ = match std::str::from_utf8(data) {
            Ok(data) => writeln!(&mut self.writer, "{}: {:?}", target, data),
            Err(..) => {
                writeln!(&mut self.writer, "{}:(bytes): {:?}", target, data)
            }
        };
    }
}

impl LogWriter for PrefixLogWriter {
    fn log_read(&mut self, data: &[u8]) {
        self.log("read", data);
    }

    fn log_write(&mut self, data: &[u8]) {
        self.log("write", data);
    }
}

/// Standard log writer does not format read and write logs.
///
/// Be aware that if you are writing data that would be masked,
/// for example, entering a password at an interactive prompt
/// the plain text value will be logged.
pub struct StandardLogWriter {
    writer: Box<dyn Write>,
}

impl Default for StandardLogWriter {
    fn default() -> Self {
        Self {
            writer: Box::new(std::io::stdout()),
        }
    }
}

impl StandardLogWriter {
    /// Create a new standard log writer.     
    pub fn new(writer: Box<dyn Write>) -> Self {
        Self { writer }
    }
}

impl LogWriter for StandardLogWriter {
    fn log_read(&mut self, data: &[u8]) {
        let _ = self.writer.write_all(data);
    }

    fn log_write(&mut self, data: &[u8]) {
        let _ = self.writer.write_all(data);
    }
}
