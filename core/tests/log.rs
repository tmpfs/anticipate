use std::{
    io::{self, prelude::*, Cursor},
    process::Command,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use anticipate::{log::PrefixLogWriter, spawn_with_options};

#[test]
#[cfg(windows)]
fn log() {
    let writer = StubWriter::default();

    let mut session = spawn_with_options(
        Command::new("python").arg("./tests/actions/cat/main.py"),
        Some(PrefixLogWriter::new(Box::new(writer.clone()))),
        None,
    )
    .unwrap();

    thread::sleep(Duration::from_millis(300));

    session.send_line("Hello World").unwrap();

    thread::sleep(Duration::from_millis(300));

    let mut buf = vec![0; 1024];
    let _ = session.read(&mut buf).unwrap();

    let bytes = writer.inner.lock().unwrap();
    let log_str = String::from_utf8_lossy(bytes.get_ref());
    assert!(log_str.as_ref().contains("write"));
    assert!(log_str.as_ref().contains("read"));
}

#[test]
#[cfg(unix)]
fn log() {
    let writer = StubWriter::default();

    let mut session = spawn_with_options(
        Command::new("cat"),
        Some(PrefixLogWriter::new(Box::new(writer.clone()))),
        None,
    )
    .unwrap();

    session.send_line("Hello World").unwrap();

    // give some time to cat
    // since sometimes we doesn't keep up to read whole string
    thread::sleep(Duration::from_millis(300));

    let mut buf = vec![0; 1024];
    let _ = session.read(&mut buf).unwrap();

    let bytes = writer.inner.lock().unwrap();
    let text = String::from_utf8_lossy(bytes.get_ref());
    assert!(
        text.contains("read") && text.contains("write"),
        "unexpected output {text:?}"
    );
}

#[test]
#[cfg(unix)]
fn log_read_line() {
    let writer = StubWriter::default();

    let mut session = spawn_with_options(
        Command::new("cat"),
        Some(PrefixLogWriter::new(Box::new(writer.clone()))),
        None,
    )
    .unwrap();

    session.send_line("Hello World").unwrap();

    let mut buf = String::new();
    let _ = session.read_line(&mut buf).unwrap();
    assert_eq!(buf, "Hello World\r\n");

    let bytes = writer.inner.lock().unwrap();
    let text = String::from_utf8_lossy(bytes.get_ref());
    if !matches!(
        text.as_ref(),
        "write: \"Hello World\\n\"\nread: \"Hello World\"\nread: \"\\r\\n\"\n"
            | "write: \"Hello World\\n\"\nread: \"Hello World\\r\\n\"\n"
            | "write: \"Hello World\"\nwrite: \"\\n\"\nread: \"Hello World\\r\\n\"\n",
    ) {
        panic!("unexpected output {text:?}");
    }
}

#[derive(Debug, Clone, Default)]
struct StubWriter {
    inner: Arc<Mutex<Cursor<Vec<u8>>>>,
}

impl Write for StubWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.lock().unwrap().flush()
    }
}
