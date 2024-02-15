use crate::{Error, Instruction, Instructions, Result, ScriptParser};
use ouroboros::self_referencing;
use probability::prelude::*;
use rexpect::{session::PtySession, spawn};
use std::{
    path::{Path, PathBuf},
    thread::{self, sleep},
    time::Duration,
};
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
}

impl Default for InterpreterOptions {
    fn default() -> Self {
        Self {
            command: "sh".to_owned(),
            timeout: Some(5000),
            cinema: None,
        }
    }
}

impl InterpreterOptions {
    /// Create interpreter options.
    pub fn new(timeout: u64) -> Self {
        Self {
            command: "sh".to_owned(),
            timeout: Some(timeout),
            cinema: None,
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
        }
    }
}

/// Script file.
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
    pub fn instructions<'s>(&'s self) -> &'s Instructions<'s> {
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
            tracing::info!(path = ?path, "parse file");
            let source = std::fs::read_to_string(&path)?;
            let source = ScriptSourceTryBuilder {
                source,
                instructions_builder: |source| ScriptParser.parse(source),
            }
            .try_build()?;

            results.push(ScriptFile { path, source });
        }
        Ok(results)
    }

    /// Execute the command and instructions in a pseudo-terminal
    /// running in a thread.
    pub fn run(&self, options: InterpreterOptions) {
        thread::scope(|s| {
            let cmd = options.command.clone();

            let handle = s.spawn(move || {
                let instructions = self.source.borrow_instructions();
                let is_cinema = options.cinema.is_some();

                if let Some(cinema) = &options.cinema {
                    // Export a vanilla shell for asciinema
                    let shell =
                        format!("PS1='{}' {}", cinema.prompt, cinema.shell);
                    std::env::set_var("SHELL", &shell);
                }

                let pragma = if let Some(Instruction::Pragma(cmd)) =
                    instructions.first()
                {
                    Some(self.resolve_path(cmd)?)
                } else {
                    None
                };

                let exec = if let (false, Some(cmd)) = (is_cinema, &pragma) {
                    cmd
                } else {
                    &cmd
                };

                tracing::info!(exec = %exec, "run");
                let mut p = spawn(exec, options.timeout)?;

                if options.cinema.is_some() {
                    p.exp_string(ASCIINEMA_WAIT)?;
                    // Wait for the initial shell prompt to flush
                    sleep(Duration::from_millis(50));
                    tracing::debug!("asciinema ready");
                }

                fn type_text(
                    p: &mut PtySession,
                    text: &str,
                    cinema: &CinemaOptions,
                ) -> Result<()> {
                    for c in UnicodeSegmentation::graphemes(text, true) {
                        p.send(c)?;
                        p.flush()?;

                        let mut source = Source(rand::rngs::OsRng);
                        let gaussian = Gaussian::new(0.0, cinema.deviation);
                        let drift = gaussian.sample(&mut source);

                        let delay = if (drift as u64) < cinema.delay {
                            let drift = drift as i64;
                            if drift < 0 {
                                cinema.delay - (drift.abs() as u64)
                            } else {
                                cinema.delay + drift as u64
                            }
                        } else {
                            cinema.delay + drift.abs() as u64
                        };

                        sleep(Duration::from_millis(delay));
                    }
                    p.send("\n")?;
                    p.flush()?;
                    Ok(())
                }

                for cmd in instructions.iter() {
                    tracing::debug!(instruction = ?cmd);
                    match cmd {
                        Instruction::Pragma(_) => {
                            if let (Some(cinema), Some(cmd)) =
                                (&options.cinema, &pragma)
                            {
                                if cinema.type_pragma {
                                    type_text(&mut p, &cmd, cinema)?;
                                } else {
                                    p.send_line(&cmd)?;
                                }
                            }
                        }
                        Instruction::Wait(delay) => {
                            sleep(Duration::from_millis(*delay));
                        }
                        Instruction::Send(line) => {
                            p.send(line.as_ref())?;
                        }
                        Instruction::SendLine(line) => {
                            let line = ScriptParser::interpolate(*line)?;
                            if let Some(cinema) = &options.cinema {
                                type_text(&mut p, line.as_ref(), cinema)?;
                            } else {
                                p.send_line(line.as_ref())?;
                            }
                        }
                        Instruction::SendControl(ctrl) => {
                            p.send_control(*ctrl)?;
                        }
                        Instruction::Expect(line) => {
                            p.exp_string(line)?;
                        }
                        Instruction::Regex(line) => {
                            p.exp_regex(line)?;
                        }
                        Instruction::ReadLine => {
                            p.read_line()?;
                        }
                        Instruction::Flush => {
                            p.flush()?;
                        }
                        Instruction::Include(_) | Instruction::Comment(_) => {}
                    }
                    sleep(Duration::from_millis(25));
                }

                if options.cinema.is_some() {
                    tracing::debug!("exit");
                    p.send_line(EXIT)?;
                } else {
                    tracing::debug!("eof");
                    p.exp_eof()?;
                }

                Ok::<(), Error>(())
            });

            let res = handle.join().unwrap();
            if let Err(e) = res {
                eprintln!("{:#?}", e);
            }
        });
    }

    fn resolve_path(&self, input: &str) -> Result<String> {
        let path = PathBuf::from(input);
        if path.is_relative() {
            if let Some(parent) = self.path.parent() {
                let new_path = parent.join(input);
                let path = new_path.canonicalize()?;
                return Ok(path.to_string_lossy().as_ref().to_owned());
            }
        }
        Ok(input.to_owned())
    }
}
