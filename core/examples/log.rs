use anticipate::{spawn, Error};

fn main() -> Result<(), Error> {
    let p = spawn("cat")?;
    let mut p = anticipate::session::log(p, std::io::stdout())?;
    p.send_line("Hello World")?;
    p.expect("Hello World")?;
    Ok(())
}
