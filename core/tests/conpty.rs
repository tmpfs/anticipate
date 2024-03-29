use std::io::Read;

#[test]
#[cfg(windows)]
fn conpty_echo() {
    let mut proc = conpty::spawn("echo Hello World").unwrap();
    let mut reader = proc.output().unwrap();
    let mut buf = [0; 1024];
    reader.read(&mut buf).unwrap();
    assert!(String::from_utf8_lossy(&buf).contains("Hello World"));
}
