use crate::{Error, Instruction, Instructions, Result, ScriptParser};
use ouroboros::self_referencing;
use rexpect::{session::PtySession, spawn, ReadUntil};
use std::{
    path::{Path, PathBuf},
    thread::{self, sleep, ScopedJoinHandle},
    time::Duration,
};
use unicode_segmentation::UnicodeSegmentation;

const ASCIINEMA_WAIT: &str =
    r#"asciinema: press <ctrl-d> or type "exit" when you're done"#;
const EXIT: &str = "exit";

/// Options for compilation.
pub struct CompileOptions {
    /// Command to execute in the pty.
    pub command: String,
    /// Timeout for rexpect.
    pub timeout: Option<u64>,
    /// Options for asciinema.
    pub cinema: Option<CinemaOptions>,
}

#[derive(Default)]
pub struct CinemaOptions {
    /// Delay in milliseconds.
    pub delay: u64,
}

impl CompileOptions {
    pub fn new_recording(output: impl AsRef<Path>, overwrite: bool) -> Self {
        let mut command =
            format!("asciinema rec {:#?}", output.as_ref().to_string_lossy());
        if overwrite {
            command.push_str(" --overwrite");
        }
        Self {
            command,
            timeout: Some(5000),
            cinema: Some(CinemaOptions { delay: 80 }),
        }
    }
}

impl Default for CompileOptions {
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
    pub fn run(&self, options: CompileOptions) {
        thread::scope(|s| {
            let cmd = options.command.clone();

            let handle = s.spawn(move || {
                let prompt = "➜ ";
                if options.cinema.is_some() {
                    let shell = format!("PS1='{}' sh -noprofile -norc", prompt);
                    std::env::set_var("SHELL", &shell);
                }

                tracing::info!(cmd = %cmd, "run");
                let mut p = spawn(&cmd, options.timeout)?;

                if options.cinema.is_some() {
                    p.exp_string(ASCIINEMA_WAIT)?;
                    // Wait for the initial shell prompt to flush
                    sleep(Duration::from_millis(250));
                    tracing::debug!("asciinema wait completed");
                }

                fn type_text(
                    p: &mut PtySession,
                    text: &str,
                    cinema: &CinemaOptions,
                ) -> Result<()> {
                    for c in UnicodeSegmentation::graphemes(text, true) {
                        p.send(c)?;
                        p.flush()?;
                        sleep(Duration::from_millis(cinema.delay));
                    }
                    p.send("\n")?;
                    p.flush()?;
                    Ok(())
                }

                let instructions =
                    self.borrow_instructions().as_ref().unwrap();
                for cmd in instructions.iter() {
                    tracing::debug!(instruction = ?cmd);
                    match cmd {
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
                        _ => {}
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
}
