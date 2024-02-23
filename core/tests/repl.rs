#![cfg(unix)]

use anticipate::{
    repl::{spawn_bash, spawn_python},
    ControlCode, Expect, WaitStatus,
};
use std::io::BufRead;
use std::{thread, time::Duration};

#[cfg(target_os = "linux")]
#[test]
fn bash() {
    let mut p = spawn_bash().unwrap();

    p.send_line("echo Hello World").unwrap();
    let mut msg = String::new();
    p.read_line(&mut msg).unwrap();
    assert!(msg.ends_with("Hello World\r\n"));

    p.send(ControlCode::EOT).unwrap();

    p.get_process_mut().exit(true).unwrap();
}

#[test]
fn python() {
    let mut p = spawn_python().unwrap();

    let prompt = p.execute("print('Hello World')").unwrap();
    let prompt = String::from_utf8_lossy(&prompt);
    assert!(prompt.contains("Hello World"), "{prompt:?}");

    thread::sleep(Duration::from_millis(300));
    p.send(ControlCode::EndOfText).unwrap();
    thread::sleep(Duration::from_millis(300));

    let mut msg = String::new();
    p.read_line(&mut msg).unwrap();
    assert!(msg.contains("\r\n"), "{msg:?}");

    let mut msg = String::new();
    p.read_line(&mut msg).unwrap();
    assert_eq!(msg, "KeyboardInterrupt\r\n");

    p.expect_prompt().unwrap();

    p.send(ControlCode::EndOfTransmission).unwrap();

    assert_eq!(
        p.get_process().wait().unwrap(),
        WaitStatus::Exited(p.get_process().pid(), 0)
    );
}

#[test]
fn bash_pwd() {
    let mut p = spawn_bash().unwrap();
    p.execute("cd /tmp/").unwrap();
    p.send_line("pwd").unwrap();
    let mut pwd = String::new();
    p.read_line(&mut pwd).unwrap();
    assert!(pwd.contains("/tmp\r\n"));
}

#[test]
fn bash_control_chars() {
    let mut p = spawn_bash().unwrap();
    p.send_line("cat <(echo ready) -").unwrap();
    thread::sleep(Duration::from_millis(300));
    p.send(ControlCode::EndOfText).unwrap(); // abort: SIGINT
    p.expect_prompt().unwrap();
    p.send_line("cat <(echo ready) -").unwrap();
    thread::sleep(Duration::from_millis(100));
    p.send(ControlCode::Substitute).unwrap(); // suspend:SIGTSTPcon
    p.expect_prompt().unwrap();
}
