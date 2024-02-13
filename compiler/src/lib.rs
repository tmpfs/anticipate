use rexpect::spawn;
use anticipate_parser::{Command, Commands};
use std::future::Future;

mod error;
pub use error::Error;
/// Result type for the compiler.
pub type Result<T> = std::result::Result<T, Error>;

pub fn compile<'s>(
    exec: &'s str,
    cmd: Commands<'s>) -> impl Future<Output = Result<()>> + 's {
    async move {
        let mut p = spawn(exec, Some(2000))?;
        for cmd in cmd.iter() {
            match cmd {
                Command::SendLine(line) => {
                    p.send_line(line)?;
                }
                Command::SendControl(ctrl) => {
                    p.send_control(*ctrl)?;
                }
                Command::Expect(line) => {
                    p.exp_string(line)?;
                }
                Command::Regex(line) => {
                    p.exp_regex(line)?;
                }
                _ => {}
            }
        }
        p.exp_eof()?;
        Ok(())
    }
}
