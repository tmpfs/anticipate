use anticipate::{spawn, ControlCode, Error, Regex};

fn main() -> Result<(), Error> {
    let mut p = spawn("ftp bks4-speedtest-1.tele2.net")?;
    p.expect(Regex("Name \\(.*\\):"))?;
    p.send_line("anonymous")?;
    p.expect("Password")?;
    p.send_line("test")?;
    p.expect("ftp>")?;
    p.send_line("cd upload")?;
    p.expect("successfully changed.")?;
    p.send_line("pwd")?;
    p.expect(Regex("[0-9]+ \"/upload\""))?;
    p.send(ControlCode::EndOfTransmission)?;
    p.expect("Goodbye.")?;
    Ok(())
}

