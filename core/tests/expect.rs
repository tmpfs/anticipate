use anticipate::{spawn, Eof, NBytes, Regex};
use std::time::Duration;

use std::io::Read;

#[cfg(unix)]
#[test]
fn expect_str() {
    let mut session = spawn("cat").unwrap();
    session.send_line("Hello World").unwrap();
    session.expect("Hello World").unwrap();
}

#[cfg(windows)]
#[test]
fn expect_str() {
    let mut session =
        spawn(r#"pwsh -c "python ./tests/actions/cat/main.py""#).unwrap();
    session.send_line("Hello World").unwrap();
    session.expect("Hello World").unwrap();
}

#[cfg(unix)]
#[test]
fn expect_regex() {
    let mut session = spawn("cat").unwrap();
    session.send_line("Hello World").unwrap();
    let m = session.expect(Regex("lo.*")).unwrap();
    assert_eq!(m.before(), b"Hel");
    assert_eq!(m.get(0).unwrap(), b"lo World\r");
}

#[cfg(unix)]
#[test]
fn expect_regex_lazy() {
    let mut session = spawn("cat").unwrap();
    session.set_expect_lazy(true);
    session.send_line("Hello World").unwrap();
    let m = session.expect(Regex("lo.*")).unwrap();
    assert_eq!(m.before(), b"Hel");
    assert_eq!(m.get(0).unwrap(), b"lo");
}

#[cfg(windows)]
#[test]
fn expect_regex() {
    let mut session = spawn("echo 'Hello World'").unwrap();
    let m = session.expect(Regex("lo.*")).unwrap();
    assert_eq!(m.matches().count(), 1);
    assert_eq!(m.get(0).unwrap(), b"lo World'\r");
}

#[cfg(unix)]
#[test]
fn expect_n_bytes() {
    let mut session = spawn("cat").unwrap();
    session.send_line("Hello World").unwrap();
    let m = session.expect(NBytes(3)).unwrap();
    assert_eq!(m.get(0).unwrap(), b"Hel");
    assert_eq!(m.before(), b"");
}

#[cfg(windows)]
#[test]
fn expect_n_bytes() {
    use anticipate::DefaultSession;
    use std::process::Command;

    let mut session = DefaultSession::spawn(Command::new(
        "python ./tests/actions/echo/main.py Hello World",
    ))
    .unwrap();
    let m = session.expect(NBytes(14)).unwrap();
    assert_eq!(m.matches().count(), 1);
    assert_eq!(m.get(0).unwrap().len(), 14);
    assert_eq!(m.before(), b"");
}

#[cfg(unix)]
#[test]
fn expect_eof() {
    let mut session = spawn("echo 'Hello World'").unwrap();
    session.set_expect_timeout(None);
    let m = session.expect(Eof).unwrap();
    assert_eq!(m.get(0).unwrap(), b"'Hello World'\r\n");
    assert_eq!(m.before(), b"");
}

#[cfg(windows)]
#[test]
#[ignore = "https://stackoverflow.com/questions/68985384/does-a-conpty-reading-pipe-get-notified-on-process-termination"]
fn expect_eof() {
    let mut session = spawn("echo 'Hello World'").unwrap();

    // give shell some time
    std::thread::sleep(Duration::from_millis(300));

    let m = session.expect(Eof).unwrap();
    assert_eq!(m.get(0).unwrap(), b"'Hello World'\r\n");
    assert_eq!(m.before(), b"");
}

#[cfg(unix)]
#[test]
fn read_after_expect_str() {
    let mut session = spawn("cat").unwrap();
    session.send_line("Hello World").unwrap();
    session.expect("Hello").unwrap();

    let mut buf = [0; 6];
    session.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b" World");
}

#[cfg(windows)]
#[test]
fn read_after_expect_str() {
    let mut session = spawn("echo 'Hello World'").unwrap();

    // give shell some time
    std::thread::sleep(Duration::from_millis(300));

    session.expect("Hello").unwrap();

    let mut buf = [0; 6];
    session.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b" World");
}

#[cfg(unix)]
#[test]
fn expect_eof_timeout() {
    let mut p = spawn("sleep 3").expect("cannot run sleep 3");
    p.set_expect_timeout(Some(Duration::from_millis(100)));
    match p.expect(Eof) {
        Err(anticipate::Error::ExpectTimeout(_)) => {}
        r => panic!("reached a timeout {r:?}"),
    }
}

#[cfg(windows)]
#[test]
fn expect_eof_timeout() {
    let mut p = spawn("sleep 3").expect("cannot run sleep 3");
    p.set_expect_timeout(Some(Duration::from_millis(100)));
    match p.expect(Eof) {
        Err(anticipate::Error::ExpectTimeout(_)) => {}
        r => panic!("should raise TimeOut {:?}", r),
    }
}
