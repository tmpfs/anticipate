//! This module contains a list of special Sessions that can be spawned.

use crate::{
    error::Error,
    session::{DefaultLogWriter, LogWriter},
    spawn, Captures, Expect, Needle, Session,
};
use std::ops::{Deref, DerefMut};

#[cfg(unix)]
use std::process::Command;

use std::io::{BufRead, Read, Write};

/// Spawn a bash session.
///
/// It uses a custom prompt to be able to controll shell better.
///
/// If you wan't to use [Session::interact] method it is better to use just Session.
/// Because we don't handle echoes here (currently). Ideally we need to.
#[cfg(unix)]
pub fn spawn_bash() -> Result<ReplSession<DefaultLogWriter>, Error> {
    const DEFAULT_PROMPT: &str = "EXPECT_PROMPT";
    let mut cmd = Command::new("bash");
    let _ = cmd.env("PS1", DEFAULT_PROMPT);
    // bind 'set enable-bracketed-paste off' turns off paste mode,
    // without it each command in bash starts and ends with an invisible sequence.
    //
    // We might need to turn it off optionally?
    let _ = cmd.env(
        "PROMPT_COMMAND",
        "PS1=EXPECT_PROMPT; unset PROMPT_COMMAND; bind 'set enable-bracketed-paste off'",
    );

    let session = crate::DefaultSession::spawn(cmd)?;

    let mut bash = ReplSession::new(
        session,
        DEFAULT_PROMPT.to_string(),
        Some("quit".to_string()),
        false,
    );

    // read a prompt to make it not available on next read.
    //
    // fix: somehow this line causes a different behaviour in iteract method.
    //      the issue most likely that with this line in interact mode ENTER produces CTRL-M
    //      when without the line it produces \r\n

    bash.expect_prompt()?;

    Ok(bash)
}

/// Spawn default python's IDLE.
pub fn spawn_python() -> Result<ReplSession<DefaultLogWriter>, Error> {
    // todo: check windows here
    // If we spawn it as ProcAttr::default().commandline("python") it will spawn processes endlessly....

    let session = spawn("python")?;

    let mut idle = ReplSession::new(
        session,
        ">>> ".to_owned(),
        Some("quit()".to_owned()),
        false,
    );
    idle.expect_prompt()?;
    Ok(idle)
}

/// Spawn a powershell session.
///
/// It uses a custom prompt to be able to controll the shell.
#[cfg(windows)]
pub fn spawn_powershell() -> Result<ReplSession, Error> {
    const DEFAULT_PROMPT: &str = "EXPECTED_PROMPT>";
    let session = spawn("pwsh -NoProfile -NonInteractive -NoLogo")?;
    let mut powershell = ReplSession::new(
        session,
        DEFAULT_PROMPT.to_owned(),
        Some("exit".to_owned()),
        true,
    );

    // https://stackoverflow.com/questions/5725888/windows-powershell-changing-the-command-prompt
    let _ = powershell.execute(format!(
        r#"function prompt {{ "{}"; return " " }}"#,
        DEFAULT_PROMPT
    ))?;

    // https://stackoverflow.com/questions/69063656/is-it-possible-to-stop-powershell-wrapping-output-in-ansi-sequences/69063912#69063912
    // https://docs.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_ansi_terminals?view=powershell-7.2#disabling-ansi-output
    let _ = powershell.execute(
        r#"[System.Environment]::SetEnvironmentVariable("TERM", "dumb")"#,
    )?;
    let _ = powershell.execute(
        r#"[System.Environment]::SetEnvironmentVariable("TERM", "NO_COLOR")"#,
    )?;

    Ok(powershell)
}

/// A repl session: e.g. bash or the python shell:
/// you have a prompt where a user inputs commands and the shell
/// which executes them and manages IO streams.
#[derive(Debug)]
pub struct ReplSession<O: LogWriter> {
    /// The prompt, used for `wait_for_prompt`,
    /// e.g. ">>> " for python.
    prompt: String,
    /// A pseudo-teletype session with a spawned process.
    session: Session<O>,
    /// A command which will be called before termination.
    quit_command: Option<String>,
    /// Flag to see if a echo is turned on.
    is_echo_on: bool,
}

impl<O: LogWriter> ReplSession<O> {
    /// Creates a repl session.
    ///
    /// The argument list is:
    ///     - session; a spawned session which repl will wrap.
    ///     - prompt; a string which will identify that the command was run.
    ///     - quit_command; a command which will be called when [ReplSession] instance is dropped.
    ///     - is_echo_on; determines whether the prompt check will be done twice.
    pub fn new(
        session: Session<O>,
        prompt: String,
        quit_command: Option<String>,
        is_echo: bool,
    ) -> Self {
        Self {
            session,
            prompt,
            quit_command,
            is_echo_on: is_echo,
        }
    }

    /// Get a used prompt.
    pub fn get_prompt(&self) -> &str {
        &self.prompt
    }

    /// Set the expected prompt.
    pub fn set_prompt(&mut self, prompt: String) {
        self.prompt = prompt
    }

    /// Get a used quit command.
    pub fn get_quit_command(&self) -> Option<&str> {
        self.quit_command.as_deref()
    }

    /// Get a echo settings.
    pub fn is_echo(&self) -> bool {
        self.is_echo_on
    }

    /// Get an inner session.
    pub fn into_session(self) -> Session<O> {
        self.session
    }
}

impl<O: LogWriter> ReplSession<O> {
    /// Block until prompt is found
    pub fn expect_prompt(&mut self) -> Result<Captures, Error> {
        self.session.expect(&self.prompt)
    }
}

impl<O: LogWriter> ReplSession<O> {
    /// Send a command to a repl and verifies that it exited.
    /// Returning it's output.
    pub fn execute<S: AsRef<str> + Clone>(
        &mut self,
        cmd: S,
    ) -> Result<Vec<u8>, Error> {
        self.send_line(cmd)?;
        let found = self.expect_prompt()?;
        Ok(found.before().to_vec())
    }

    /// Sends line to repl (and flush the output).
    ///
    /// If echo_on=true wait for the input to appear.
    pub fn send_line<Text: AsRef<str>>(
        &mut self,
        line: Text,
    ) -> Result<(), Error> {
        let text = line.as_ref();
        self.session.send_line(text)?;
        if self.is_echo_on {
            let _ = self.expect(line.as_ref())?;
        }
        Ok(())
    }

    /// Send a quit command.
    ///
    /// In async version we it won't be send on Drop so,
    /// If you wan't it to be send you must do it yourself.
    pub fn exit(&mut self) -> Result<(), Error> {
        if let Some(quit_command) = &self.quit_command {
            self.session.send_line(quit_command)?;
        }

        Ok(())
    }
}

impl<O: LogWriter> Deref for ReplSession<O> {
    type Target = Session<O>;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl<O: LogWriter> DerefMut for ReplSession<O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.session
    }
}

impl<O: LogWriter> Expect for ReplSession<O> {
    fn send<B: AsRef<[u8]>>(&mut self, buf: B) -> std::io::Result<()> {
        self.session.send(buf)
    }

    fn send_line(&mut self, text: &str) -> std::io::Result<()> {
        self.session.send_line(text)
    }

    fn expect<N>(&mut self, needle: N) -> std::result::Result<Captures, Error>
    where
        N: Needle,
    {
        self.session.expect(needle)
    }
}

impl<O: LogWriter> Write for ReplSession<O> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.session.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.session.flush()
    }
}

impl<O: LogWriter> BufRead for ReplSession<O> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.session.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.session.consume(amt)
    }
}

impl<O: LogWriter> Read for ReplSession<O> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.session.read(buf)
    }
}
