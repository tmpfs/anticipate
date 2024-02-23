#![cfg(unix)]

use anticipate::{spawn, Eof, NBytes, Regex, WaitStatus};
use std::thread;
use std::time::Duration;

#[cfg(unix)]
#[test]
fn is_matched_str() {
    let mut session = spawn("cat").unwrap();
    session.send_line("Hello World").unwrap();
    thread::sleep(Duration::from_millis(600));
    assert!(session.is_matched("Hello World").unwrap());
}

#[cfg(unix)]
#[test]
fn is_matched_regex() {
    let mut session = spawn("cat").unwrap();
    session.send_line("Hello World").unwrap();

    thread::sleep(Duration::from_millis(600));

    assert!(session.is_matched(Regex("lo.*")).unwrap());
}

#[cfg(unix)]
#[test]
fn is_matched_bytes() {
    let mut session = spawn("cat").unwrap();
    session.send_line("Hello World").unwrap();

    thread::sleep(Duration::from_millis(600));

    assert!(session.is_matched(NBytes(3)).unwrap());
}

#[cfg(target_os = "linux")]
#[test]
fn is_matched_eof() {
    let mut session = spawn("echo 'Hello World'").unwrap();

    assert_eq!(
        session.get_process().wait().unwrap(),
        WaitStatus::Exited(session.get_process().pid(), 0),
    );

    assert!(session.is_matched(Eof).unwrap());
}

#[cfg(unix)]
#[test]
fn read_after_is_matched() {
    use std::io::Read;

    let mut session = spawn("cat").unwrap();
    session.send_line("Hello World").unwrap();

    thread::sleep(Duration::from_millis(600));

    assert!(session.is_matched("Hello").unwrap());

    // we stop process so read operation will end up with EOF.
    // other wise read call would block.
    session.get_process_mut().exit(false).unwrap();

    let mut buf = [0; 128];
    let n = session.read(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"Hello World\r\n");
}

#[cfg(target_os = "linux")]
#[test]
fn check_after_is_matched_eof() {
    let mut p = spawn("echo AfterSleep").expect("cannot run echo");
    assert_eq!(
        WaitStatus::Exited(p.get_process().pid(), 0),
        p.get_process().wait().unwrap()
    );
    assert!(p.is_matched(Eof).unwrap());

    let m = p.check(Eof).unwrap();

    #[cfg(target_os = "linux")]
    assert_eq!(m.get(0).unwrap(), b"AfterSleep\r\n");

    #[cfg(not(target_os = "linux"))]
    assert_eq!(m.get(0).unwrap(), b"");
}

#[cfg(target_os = "linux")]
#[test]
fn expect_after_is_matched_eof() {
    let mut p = spawn("echo AfterSleep").expect("cannot run echo");
    assert_eq!(
        WaitStatus::Exited(p.get_process().pid(), 0),
        p.get_process().wait().unwrap()
    );
    assert!(p.is_matched(Eof).unwrap());

    let m = p.expect(Eof).unwrap();

    #[cfg(target_os = "linux")]
    assert_eq!(m.get(0).unwrap(), b"AfterSleep\r\n");

    #[cfg(not(target_os = "linux"))]
    assert_eq!(m.get(0).unwrap(), b"");

    assert!(matches!(p.expect("").unwrap_err(), anticipate::Error::Eof));
}
