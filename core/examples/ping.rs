#[cfg(unix)]
use anticipate::{repl::spawn_bash, ControlCode, Error, Expect};

#[cfg(unix)]
fn main() -> Result<(), Error> {
    let mut p = spawn_bash()?;
    p.send_line("ping 8.8.8.8")?;
    p.expect("bytes of data")?;
    p.send(ControlCode::try_from("^Z").unwrap())?;
    p.expect_prompt()?;
    // bash writes 'ping 8.8.8.8' to stdout again to state which job was put into background
    p.send_line("bg")?;
    p.expect("ping 8.8.8.8")?;
    p.expect_prompt()?;
    p.send_line("sleep 0.5")?;
    p.expect_prompt()?;
    // bash writes 'ping 8.8.8.8' to stdout again to state which job was put into foreground
    p.send_line("fg")?;
    p.expect("ping 8.8.8.8")?;
    p.send(ControlCode::try_from("^D").unwrap())?;
    p.expect("packet loss")?;

    Ok(())
}

#[cfg(windows)]
fn main() {
    panic!("An example doesn't supported on windows")
}
