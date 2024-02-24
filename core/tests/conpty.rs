use std::io::prelude::*;

#[test]
#[cfg(windows)]
fn conpty_echo() {
    let mut proc = conpty::spawn("echo Hello World").unwrap();
    proc.set_echo(true).unwrap();
    let mut reader = proc.output().unwrap();
    //let mut writer = proc.output().unwrap();

    let mut buf = [0; 1028];
    let n = reader.read(&mut buf).unwrap();

    assert!(String::from_utf8_lossy(&buf).contains("Hello World"));
    
    println!("READ FROM THE PROGRAM OUTPUT");

}
