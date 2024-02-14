use crate::{Error, Instruction, Instructions, Result, ScriptParser};
use ouroboros::self_referencing;
use rexpect::{session::PtySession, spawn};
use std::{
    path::{Path, PathBuf},
    thread::{self, sleep},
    time::Duration,
};
use unicode_segmentation::UnicodeSegmentation;
use probability::prelude::*;

struct Source<T>(T);

impl<T: rand::RngCore> source::Source for Source<T> {
    fn read_u64(&mut self) -> u64 {
        self.0.next_u64()
    }
}

const ASCIINEMA_WAIT: &str =
    r#"asciinema: press <ctrl-d> or type "exit" when you're done"#;
const EXIT: &str = "exit";

/// Options for compilation.
pub struct InterpreterOptions {
    /// Command to execute in the pty.
    pub command: String,
    /// Timeout for rexpect.
    pub timeout: Option<u64>,
    /// Options for asciinema.
    pub cinema: Option<CinemaOptions>,
}

pub struct CinemaOptions {
    /// Delay in milliseconds.
    pub delay: u64,
    /// Type pragma command.
    pub type_pragma: bool,
    /// Deviation for gaussian delay modification.
    pub deviation: f64,
}

impl Default for CinemaOptions {
    fn default() -> Self {
        Self {
            delay: 80,
            type_pragma: false,
            deviation: 5.0,
        }
    }
}

impl InterpreterOptions {
    pub fn new_recording(
        output: impl AsRef<Path>,
        overwrite: bool,
        options: CinemaOptions,
    ) -> Self {
        let mut command = format!(
            "asciinema rec {:#?}",
            output.as_ref().to_string_lossy(),
        );
        if overwrite {
            command.push_str(" --overwrite");
        }
        Self {
            command,
            timeout: Some(5000),
            cinema: Some(options),
        }
    }
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

/// Script file.
#[self_referencing]
#[derive(Debug)]
pub struct ScriptFile {
    /// Path to the source file.
    pub path: PathBuf,
    /// Script source.
    pub source: String,
    /// Parsed instructions.
    #[borrows(source)]
    #[covariant]
    pub instructions: Result<Instructions<'this>>,
}

impl ScriptFile {
    /// Parse a collection of files.
    pub fn parse_files(paths: Vec<PathBuf>) -> Result<Vec<ScriptFile>> {
        let mut results = Vec::new();
        for path in paths {
            tracing::info!(path = ?path, "parse file");
            let source = std::fs::read_to_string(&path)?;
            let script = ScriptFileBuilder {
                path,
                source,
                instructions_builder: |source| ScriptParser.parse(source),
            }
            .build();

            if let Err(e) = script.borrow_instructions() {
                return Err(Error::Message(e.to_string()));
            }

            results.push(script);
        }
        Ok(results)
    }

    /// Execute the pty command and instructions in a thread.
    pub fn run(&self, options: InterpreterOptions) {
        thread::scope(|s| {
            let cmd = options.command.clone();

            let handle = s.spawn(move || {
                let prompt = "➜ ";
                let instructions =
                    self.borrow_instructions().as_ref().unwrap();
                let is_cinema = options.cinema.is_some();

                if is_cinema {
                    // Export a vanilla shell for asciinema
                    let shell =
                        format!("PS1='{}' sh -noprofile -norc", prompt);
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
                    sleep(Duration::from_millis(250));
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
                        Instruction::SendLine(line) => {
                            if let Some(cinema) = &options.cinema {
                                type_text(&mut p, line, cinema)?;
                            } else {
                                p.send_line(line)?;
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
            if let Some(parent) = self.borrow_path().parent() {
                let new_path = parent.join(input);
                let path = new_path.canonicalize()?;
                return Ok(path.to_string_lossy().as_ref().to_owned());
            }
        }
        Ok(input.to_owned())
    }
}
