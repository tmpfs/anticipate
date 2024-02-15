use crate::{
    resolve_path, Error, Instruction, Instructions, Result, ScriptParser,
};
use ouroboros::self_referencing;
use probability::prelude::*;
use rexpect::{session::PtySession, spawn};
use std::{
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};
use tracing::{span, Level};
use unicode_segmentation::UnicodeSegmentation;

struct Source<T>(T);

impl<T: rand::RngCore> source::Source for Source<T> {
    fn read_u64(&mut self) -> u64 {
        self.0.next_u64()
    }
}

const ASCIINEMA_WAIT: &str =
    r#"asciinema: press <ctrl-d> or type "exit" when you're done"#;
const EXIT: &str = "exit";

/// Options for asciinema execution.
#[derive(Debug, Clone)]
pub struct CinemaOptions {
    /// Delay in milliseconds.
    pub delay: u64,
    /// Type pragma command.
    pub type_pragma: bool,
    /// Deviation for gaussian delay modification.
    pub deviation: f64,
    /// Prompt for the shell.
    pub prompt: String,
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
            deviation: 5.0,
            prompt: "➜ ".to_string(),
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
    /// Echo to stdout.
    pub echo: bool,
}

impl Default for InterpreterOptions {
    fn default() -> Self {
        Self {
            command: "sh".to_owned(),
            //command: "PS1='$ ' sh -noprofile -norc".to_owned(),
            timeout: Some(5000),
            cinema: None,
            id: None,
            echo: true,
        }
    }
}

impl InterpreterOptions {
    /// Create interpreter options.
    pub fn new(timeout: u64) -> Self {
        Self {
            command: "sh".to_owned(),
            //command: "PS1='$ ' sh -noprofile -norc".to_owned(),
            timeout: Some(timeout),
            cinema: None,
            id: None,
            echo: true,
        }
    }

    /// Create interpreter options for asciinema recording.
    pub fn new_recording(
        output: impl AsRef<Path>,
        overwrite: bool,
        options: CinemaOptions,
        timeout: u64,
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
            timeout: Some(timeout),
            cinema: Some(options),
            id: None,
            echo: true,
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

        if let Some(cinema) = &options.cinema {
            // Export a vanilla shell for asciinema
            let shell = format!("PS1='{}' {}", cinema.prompt, cinema.shell);
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
        let mut p = spawn(&exec_cmd, options.timeout)?;

        if options.cinema.is_some() {
            p.exp_string(ASCIINEMA_WAIT)?;
            // Wait for the initial shell prompt to flush
            sleep(Duration::from_millis(50));
            tracing::debug!("asciinema ready");
        }

        fn type_text(
            pty: &mut PtySession,
            text: &str,
            cinema: &CinemaOptions,
            echo: bool,
        ) -> Result<()> {
            for c in UnicodeSegmentation::graphemes(text, true) {
                pty.send(c)?;
                pty.flush()?;

                if echo {
                    //println!("> {}", c);
                }

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

            if echo {
                //println!("> {}", '\n');
            }

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
                                type_text(p, cmd, cinema, options.echo)?;
                            } else {
                                if options.echo {
                                    //println!("> {}", cmd);
                                }
                                p.send_line(cmd)?;
                            }
                        }
                    }
                    Instruction::Wait(delay) => {
                        sleep(Duration::from_millis(*delay));
                    }
                    Instruction::Send(line) => {
                        if options.echo {
                            //println!("> {}", line);
                        }
                        p.send(line.as_ref())?;
                    }
                    Instruction::SendLine(line) => {
                        let line = ScriptParser::interpolate(line)?;
                        if let Some(cinema) = &options.cinema {
                            type_text(p, line.as_ref(), cinema, options.echo)?;
                        } else {
                            if options.echo {
                                //println!("> {}", line);
                            }
                            p.send_line(line.as_ref())?;
                        }
                    }
                    Instruction::SendControl(ctrl) => {
                        p.send_control(*ctrl)?;
                    }
                    Instruction::Expect(line) => {
                        let output = p.exp_string(line)?;
                        if options.echo {
                            //println!("< {}", output);
                        }
                    }
                    Instruction::Regex(line) => {
                        let (output, re) = p.exp_regex(line)?;
                        if options.echo {
                            //println!("< {}", output);
                        }
                    }
                    Instruction::ReadLine => {
                        let line = p.read_line()?;
                        if options.echo {
                            //println!("< {}", line);
                        }
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

                //sleep(Duration::from_millis(100));
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
            p.send_line(EXIT)?;
        } else {
            tracing::debug!("eof");
            p.exp_eof()?;
        }

        Ok(())
    }
}
