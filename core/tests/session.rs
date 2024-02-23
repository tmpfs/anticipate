use anticipate::{spawn, DefaultSession};

#[cfg(not(windows))]
use std::io::{Read, Write};

#[cfg(unix)]
#[test]
fn send() {
    let mut session = spawn("cat").unwrap();
    session.send("Hello World").unwrap();

    session.write_all(&[3]).unwrap(); // Ctrl+C
    session.flush().unwrap();

    let mut buf = String::new();
    session.read_to_string(&mut buf).unwrap();

    // cat doesn't printed anything
    assert_eq!(buf, "");
}

#[cfg(windows)]
#[test]
fn send() {
    use std::io::Write;

    let mut session = spawn("python ./tests/actions/cat/main.py").unwrap();
    session.write(b"Hello World").unwrap();
    session.expect("Hello World").unwrap();
}

#[cfg(unix)]
#[test]
fn send_multiline() {
    let mut session = spawn("cat").unwrap();
    session.send("Hello World\n").unwrap();

    let m = session.expect('\n').unwrap();
    let buf = String::from_utf8_lossy(m.before());

    assert_eq!(buf, "Hello World\r");

    session.get_process_mut().exit(true).unwrap();
}

#[cfg(windows)]
#[test]
fn send_multiline() {
    let mut session = spawn("python ./tests/actions/cat/main.py").unwrap();
    session.send("Hello World\r\n").unwrap();
    let m = session.expect('\n').unwrap();
    let buf = String::from_utf8_lossy(m.before());
    assert!(buf.contains("Hello World"), "{:?}", buf);
    session.get_process_mut().exit(0).unwrap();
}

#[cfg(unix)]
#[test]
fn send_line() {
    let mut session = spawn("cat").unwrap();
    session.send_line("Hello World").unwrap();

    let m = session.expect('\n').unwrap();
    let buf = String::from_utf8_lossy(m.before());

    assert_eq!(buf, "Hello World\r");

    session.get_process_mut().exit(true).unwrap();
}

#[cfg(windows)]
#[test]
fn send_line() {
    let mut session = spawn("python ./tests/actions/cat/main.py").unwrap();
    session.send_line("Hello World").unwrap();
    let m = session.expect('\n').unwrap();
    let buf = String::from_utf8_lossy(m.before());
    assert!(buf.contains("Hello World"), "{:?}", buf);
    session.get_process_mut().exit(0).unwrap();
}

#[test]
fn test_spawn_no_command() {
    #[cfg(unix)]
    assert!(spawn("").is_err());
    #[cfg(windows)]
    assert!(spawn("").is_ok());
}

#[test]
#[ignore = "it's a compile time check"]
fn test_session_as_writer() {
    let _: Box<dyn std::io::Write> = Box::new(spawn("ls").unwrap());
    let _: Box<dyn std::io::Read> = Box::new(spawn("ls").unwrap());
    let _: Box<dyn std::io::BufRead> = Box::new(spawn("ls").unwrap());

    fn _io_copy(mut session: DefaultSession) {
        let _ = std::io::copy(&mut std::io::empty(), &mut session).unwrap();
    }
}
