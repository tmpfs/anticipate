use crate::{
    resolve_path, Error, Instruction, Instructions, Result, ScriptParser,
};
use expectrl::{
    process::unix::{PtyStream, UnixProcess},
    session::{log, Session},
    stream::log::LogStream,
    Captures, ControlCode, Eof, Needle, Regex, WaitStatus,
};
use ouroboros::self_referencing;
use probability::prelude::*;
use std::io::{self, BufRead, Read, Stdout, Write};
use std::{
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};
use tracing::{span, Level};
use unicode_segmentation::UnicodeSegmentation;

const ASCIINEMA_WAIT: &str =
    r#"asciinema: press <ctrl-d> or type "exit" when you're done"#;
const PROMPT: &str = "➜ ";

struct Source<T>(T);

impl<T: rand::RngCore> source::Source for Source<T> {
    fn read_u64(&mut self) -> u64 {
        self.0.next_u64()
    }
}

/// Options for asciinema execution.
#[derive(Debug, Clone)]
pub struct CinemaOptions {
    /// Delay in milliseconds.
    pub delay: u64,
    /// Type pragma command.
    pub type_pragma: bool,
    /// Deviation for gaussian delay modification.
    pub deviation: f64,
    /// Shell to run.
    pub shell: String,
    /// Terminal columns.
    pub cols: u64,
    /// Terminal rows.
    pub rows: u64,
}

impl Default for CinemaOptions {
    fn default() -> Self {
        Self {
            delay: 80,
            type_pragma: false,
            deviation: 15.0,
            shell: "sh -noprofile -norc".to_string(),
            cols: 80,
            rows: 24,
        }
    }
}

/// Options for the interpreter.
pub struct InterpreterOptions {
    /// Command to execute in the pty.
    pub command: String,
    /// Timeout for rexpect.
    pub timeout: Option<u64>,
    /// Options for asciinema.
    pub cinema: Option<CinemaOptions>,
    /// Identifier.
    pub id: Option<String>,
    /// Prompt.
    pub prompt: Option<String>,
    /// Echo to stdout.
    pub echo: bool,
}

impl Default for InterpreterOptions {
    fn default() -> Self {
        Self {
            command: "sh -noprofile -norc".to_owned(),
            prompt: None,
            //command: "PS1='> ' sh".to_owned(),
            timeout: Some(5000),
            cinema: None,
            id: None,
            echo: false,
        }
    }
}

impl InterpreterOptions {
    /// Create interpreter options.
    pub fn new(timeout: u64, echo: bool) -> Self {
        Self {
            command: "sh -noprofile -norc".to_owned(),
            prompt: None,
            //command: "bash".to_owned(),
            //command: "PS1='> ' sh".to_owned(),
            //command: "PS1='$ ' sh -noprofile -norc".to_owned(),
            timeout: Some(timeout),
            cinema: None,
            id: None,
            echo,
        }
    }

    /// Create interpreter options for asciinema recording.
    pub fn new_recording(
        output: impl AsRef<Path>,
        overwrite: bool,
        options: CinemaOptions,
        timeout: u64,
        echo: bool,
    ) -> Self {
        let mut command = format!(
            "asciinema rec {:#?}",
            output.as_ref().to_string_lossy(),
        );
        if overwrite {
            command.push_str(" --overwrite");
        }
        command.push_str(&format!(" --rows={}", options.rows));
        command.push_str(&format!(" --cols={}", options.cols));
        Self {
            command,
            prompt: None,
            timeout: Some(timeout),
            cinema: Some(options),
            id: None,
            echo,
        }
    }
}

/// Script file.
#[derive(Debug)]
pub struct ScriptFile {
    path: PathBuf,
    source: ScriptSource,
}

impl ScriptFile {
    /// Path to the source file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Source contents of the file.
    pub fn source(&self) -> &str {
        self.source.borrow_source()
    }

    /// Script instructions.
    pub fn instructions(&self) -> &Instructions<'_> {
        self.source.borrow_instructions()
    }
}

#[self_referencing]
#[derive(Debug)]
/// Script file.
pub struct ScriptSource {
    /// Script source.
    pub source: String,
    /// Parsed instructions.
    #[borrows(source)]
    #[covariant]
    pub instructions: Instructions<'this>,
}

impl ScriptFile {
    /// Parse a collection of files.
    pub fn parse_files(paths: Vec<PathBuf>) -> Result<Vec<ScriptFile>> {
        let mut results = Vec::new();
        for path in paths {
            let script = Self::parse(path)?;
            results.push(script);
        }
        Ok(results)
    }

    /// Parse a single file.
    pub fn parse(path: impl AsRef<Path>) -> Result<ScriptFile> {
        let source = Self::parse_source(path.as_ref())?;
        Ok(ScriptFile {
            path: path.as_ref().to_owned(),
            source,
        })
    }

    fn parse_source(path: impl AsRef<Path>) -> Result<ScriptSource> {
        let mut includes = Vec::new();
        let source = std::fs::read_to_string(path.as_ref())?;
        let mut source = ScriptSourceTryBuilder {
            source,
            instructions_builder: |source| {
                let (instructions, mut file_includes) =
                    ScriptParser::parse_file(source, path.as_ref())?;
                includes.append(&mut file_includes);
                Ok::<_, Error>(instructions)
            },
        }
        .try_build()?;

        for raw in includes {
            let src = Self::parse_source(&raw.path)?;
            let instruction = Instruction::Include(src);
            source.with_instructions_mut(|i| {
                if raw.index < i.len() {
                    i.insert(raw.index, instruction);
                } else {
                    i.push(instruction);
                }
            });
        }

        Ok(source)
    }

    /// Execute the command and instructions in a pseudo-terminal
    /// running in a thread.
    pub fn run(&self, options: InterpreterOptions) -> Result<()> {
        let cmd = options.command.clone();

        let span = if let Some(id) = &options.id {
            span!(Level::DEBUG, "run", id = id)
        } else {
            span!(Level::DEBUG, "run")
        };

        let _enter = span.enter();

        let instructions = self.source.borrow_instructions();
        let is_cinema = options.cinema.is_some();

        let prompt =
            options.prompt.clone().unwrap_or_else(|| PROMPT.to_owned());
        std::env::set_var("PS1", &prompt);

        if let Some(cinema) = &options.cinema {
            // Export a vanilla shell for asciinema
            let shell = format!("PS1='{}' {}", &prompt, cinema.shell);
            std::env::set_var("SHELL", shell);
        }

        let pragma =
            if let Some(Instruction::Pragma(cmd)) = instructions.first() {
                Some(resolve_path(&self.path, cmd)?)
            } else {
                None
            };

        let exec_cmd = if let (false, Some(pragma)) = (is_cinema, &pragma) {
            pragma.as_ref().to_owned()
        } else {
            cmd.to_owned()
        };

        tracing::info!(exec = %exec_cmd, "run");
        let mut p = session(
            &exec_cmd,
            options.timeout,
            prompt,
            options.echo,
        )?;

        if options.cinema.is_some() {
            p.expect(ASCIINEMA_WAIT)?;
            // Wait for the initial shell prompt to flush
            sleep(Duration::from_millis(50));
            tracing::debug!("asciinema ready");
        }

        fn type_text(
            pty: &mut PtySession,
            text: &str,
            cinema: &CinemaOptions,
        ) -> Result<()> {
            for c in UnicodeSegmentation::graphemes(text, true) {
                pty.send(c)?;
                pty.flush()?;

                let mut source = Source(rand::rngs::OsRng);
                let gaussian = Gaussian::new(0.0, cinema.deviation);
                let drift = gaussian.sample(&mut source);

                let delay = if (drift as u64) < cinema.delay {
                    let drift = drift as i64;
                    if drift < 0 {
                        cinema.delay - drift.unsigned_abs()
                    } else {
                        cinema.delay + drift as u64
                    }
                } else {
                    cinema.delay + drift.abs() as u64
                };

                sleep(Duration::from_millis(delay));
            }

            pty.send("\n")?;
            pty.flush()?;

            Ok(())
        }

        fn exec(
            p: &mut PtySession,
            instructions: &[Instruction<'_>],
            options: &InterpreterOptions,
            pragma: Option<&str>,
        ) -> Result<()> {
            for cmd in instructions.iter() {
                tracing::debug!(instruction = ?cmd);
                match cmd {
                    Instruction::Pragma(_) => {
                        if let (Some(cinema), Some(cmd)) =
                            (&options.cinema, &pragma)
                        {
                            if cinema.type_pragma {
                                type_text(p, cmd, cinema)?;
                            } else {
                                p.send_line(cmd)?;
                            }
                        }
                    }
                    Instruction::Wait(delay) => {
                        sleep(Duration::from_millis(*delay));
                    }
                    Instruction::Send(line) => {
                        p.send(line)?;
                    }
                    Instruction::SendLine(line) => {
                        let line = ScriptParser::interpolate(line)?;
                        if let Some(cinema) = &options.cinema {
                            type_text(p, line.as_ref(), cinema)?;
                        } else {
                            p.send_line(line.as_ref())?;
                        }
                    }
                    Instruction::SendControl(ctrl) => {
                        let ctrl =
                            ControlCode::try_from(*ctrl).map_err(|_| {
                                Error::InvalidControlCode(ctrl.to_string())
                            })?;
                        p.send(ctrl)?;
                    }
                    Instruction::Expect(line) => {
                        p.expect(line)?;
                    }
                    Instruction::Regex(line) => {
                        p.expect(Regex(line))?;
                    }
                    Instruction::ReadLine => {
                        let mut line = String::new();
                        p.read_line(&mut line)?;
                    }
                    Instruction::WaitPrompt => {
                        //p.wait_for_prompt()?;
                    }
                    Instruction::Flush => {
                        p.flush()?;
                    }
                    Instruction::Comment(_) => {}
                    Instruction::Include(source) => {
                        exec(
                            p,
                            source.borrow_instructions(),
                            options,
                            pragma,
                        )?;
                    }
                }

                sleep(Duration::from_millis(25));
            }
            Ok(())
        }

        exec(
            &mut p,
            instructions,
            &options,
            pragma.as_ref().map(|i| i.as_ref()),
        )?;

        if options.cinema.is_some() {
            tracing::debug!("exit");
            //p.send_control('d')?;

            p.send(ControlCode::EndOfTransmission)?;
        } else {
            tracing::debug!("eof");
            p.send(ControlCode::EndOfTransmission)?;
        }

        Ok(())
    }
}

fn session(
    cmd: &str,
    _timeout: Option<u64>,
    _prompt: String,
    echo: bool,
) -> Result<PtySession> {
    use std::process::Command;
    let mut parts = comma::parse_command(cmd)
        .ok_or(Error::BadArguments(cmd.to_owned()))?;
    let prog = parts.remove(0);
    let mut command = Command::new(prog);
    command.args(parts);

    let pty = Session::spawn(command)?;
    let session = if echo {
        PtySession::Logged(log(pty, std::io::stdout())?)
    } else {
        PtySession::Default(pty)
    };

    Ok(session)
}

type LogSession = Session<UnixProcess, LogStream<PtyStream, Stdout>>;

pub enum PtySession {
    Default(Session),
    Logged(LogSession),
}

impl PtySession {
    pub fn send<B: AsRef<[u8]>>(&mut self, buf: B) -> io::Result<()> {
        match self {
            PtySession::Default(s) => s.send(buf),
            PtySession::Logged(s) => s.send(buf),
        }
    }

    pub fn send_line(&mut self, text: &str) -> io::Result<()> {
        match self {
            PtySession::Default(s) => s.send_line(text),
            PtySession::Logged(s) => s.send_line(text),
        }
    }

    pub fn expect<N>(
        &mut self,
        needle: N,
    ) -> std::result::Result<Captures, expectrl::Error>
    where
        N: Needle,
    {
        match self {
            PtySession::Default(s) => s.expect(needle),
            PtySession::Logged(s) => s.expect(needle),
        }
    }
}

impl Write for PtySession {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            PtySession::Default(s) => s.write(buf),
            PtySession::Logged(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            PtySession::Default(s) => s.flush(),
            PtySession::Logged(s) => s.flush(),
        }
    }
}

impl BufRead for PtySession {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self {
            PtySession::Default(s) => s.fill_buf(),
            PtySession::Logged(s) => s.fill_buf(),
        }
    }

    fn consume(&mut self, amt: usize) {
        match self {
            PtySession::Default(s) => s.consume(amt),
            PtySession::Logged(s) => s.consume(amt),
        }
    }
}

impl Read for PtySession {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            PtySession::Default(s) => s.read(buf),
            PtySession::Logged(s) => s.read(buf),
        }
    }
}
