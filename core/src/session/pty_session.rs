mod sync {
    use crate::{
        session::{DefaultLogSession, OsProcess, Session, TeeLogSession},
        Captures, Expect, Needle,
    };
    use std::io::{self, BufRead, Read, Write};

    /// Wraps a session that may be logged to stdout.
    #[derive(Debug)]
    pub enum PtySession {
        /// Default pty session.
        Default(Session),
        /// Pty session that logs formatted output to stdout.
        Logger(DefaultLogSession),
        /// Pty session that passes through I/O to stdout.
        TeeLogger(TeeLogSession),
    }

    impl PtySession {
        /// Get a reference to a process running program.
        pub fn get_process(&self) -> &OsProcess {
            match self {
                PtySession::Default(s) => s.get_process(),
                PtySession::Logger(s) => s.get_process(),
                PtySession::TeeLogger(s) => s.get_process(),
            }
        }
    }

    impl Expect for PtySession {
        fn send<B: AsRef<[u8]>>(&mut self, buf: B) -> io::Result<()> {
            match self {
                PtySession::Default(s) => s.send(buf),
                PtySession::Logger(s) => s.send(buf),
                PtySession::TeeLogger(s) => s.send(buf),
            }
        }

        fn send_line(&mut self, text: &str) -> io::Result<()> {
            match self {
                PtySession::Default(s) => s.send_line(text),
                PtySession::Logger(s) => s.send_line(text),
                PtySession::TeeLogger(s) => s.send_line(text),
            }
        }

        fn expect<N>(&mut self, needle: N) -> Result<Captures, crate::Error>
        where
            N: Needle,
        {
            match self {
                PtySession::Default(s) => s.expect(needle),
                PtySession::Logger(s) => s.expect(needle),
                PtySession::TeeLogger(s) => s.expect(needle),
            }
        }
    }

    impl Write for PtySession {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            match self {
                PtySession::Default(s) => s.write(buf),
                PtySession::Logger(s) => s.write(buf),
                PtySession::TeeLogger(s) => s.write(buf),
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            match self {
                PtySession::Default(s) => s.flush(),
                PtySession::Logger(s) => s.flush(),
                PtySession::TeeLogger(s) => s.flush(),
            }
        }
    }

    impl BufRead for PtySession {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            match self {
                PtySession::Default(s) => s.fill_buf(),
                PtySession::Logger(s) => s.fill_buf(),
                PtySession::TeeLogger(s) => s.fill_buf(),
            }
        }

        fn consume(&mut self, amt: usize) {
            match self {
                PtySession::Default(s) => s.consume(amt),
                PtySession::Logger(s) => s.consume(amt),
                PtySession::TeeLogger(s) => s.consume(amt),
            }
        }
    }

    impl Read for PtySession {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            match self {
                PtySession::Default(s) => s.read(buf),
                PtySession::Logger(s) => s.read(buf),
                PtySession::TeeLogger(s) => s.read(buf),
            }
        }
    }
}

pub use sync::PtySession;
