//! Script automation tool with support for recording
//! using [asciinema](https://asciinema.org/).
//!
//! For programmatic access use the [anticipate-core](https://docs.rs/anticipate-core) crate, see [the repository](https://github.com/tmpfs/anticipate/) for examples.
use anticipate_core::{CinemaOptions, InterpreterOptions, ScriptFile};
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use rayon::prelude::*;
use std::path::PathBuf;
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[doc(hidden)]
const LOG_FILE_NAME: &str = "anticipate.log";

#[doc(hidden)]
fn main() -> Result<()> {
    if let Err(e) = start() {
        tracing::error!(error = ?e);
        std::process::exit(1);
    }
    Ok(())
}

#[doc(hidden)]
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Anticipate {
    #[clap(subcommand)]
    cmd: Command,
}

#[doc(hidden)]
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Parse scripts and print the instructions.
    Parse {
        /// Directory to write logs.
        #[clap(short, long)]
        logs: Option<PathBuf>,

        /// Parse scripts in parallel.
        #[clap(short, long)]
        parallel: bool,

        /// Input file paths.
        input: Vec<PathBuf>,
    },
    /// Run scripts.
    Run {
        /// Directory to write logs.
        #[clap(short, long)]
        logs: Option<PathBuf>,

        /// Execute scripts in parallel.
        #[clap(short, long)]
        parallel: bool,

        /// Timeout for the pseudo-terminal.
        #[clap(short, long, default_value = "5000")]
        timeout: u64,

        /// Input file paths.
        input: Vec<PathBuf>,
    },

    /// Record using asciinema.
    #[clap(alias = "rec")]
    Record {
        /// Directory to write logs.
        #[clap(short, long)]
        logs: Option<PathBuf>,

        /// Execute scripts in parallel.
        #[clap(short, long)]
        parallel: bool,

        /// Timeout for the pseudo-terminal.
        #[clap(short, long, default_value = "5000")]
        timeout: u64,

        /// Overwrite existing recordings.
        #[clap(short, long)]
        overwrite: bool,

        /// Delay between keystrokes.
        #[clap(short, long, default_value = "80")]
        delay: u64,

        /// Standard deviation for gaussian distribution.
        #[clap(long, default_value = "5.0")]
        deviation: f64,

        /// Prompt for the shell.
        #[clap(long, default_value = "➜ ")]
        prompt: String,

        /// Shell command.
        #[clap(long, default_value = "sh -noprofile -norc")]
        shell: String,

        /// Type pragma commands.
        #[clap(long)]
        type_pragma: bool,

        /// Number of lines to trim from end of recording.
        #[clap(long, default_value = "2")]
        trim_lines: u64,

        /// Number of terminal columns.
        #[clap(long, default_value = "80")]
        cols: u64,

        /// Number of terminal rows.
        #[clap(long, default_value = "24")]
        rows: u64,

        /// Directory for recordings.
        output: PathBuf,

        /// Input file paths.
        input: Vec<PathBuf>,
    },
}

#[doc(hidden)]
fn start() -> Result<()> {
    let args = Anticipate::parse();
    match args.cmd {
        Command::Parse {
            input,
            logs,
            parallel,
        } => {
            if let Some(logs) = logs {
                init_subscriber(logs, None)?;
            }

            let mut files = Vec::new();
            for file in input {
                if !file.exists() {
                    bail!("file {} does not exist", file.to_string_lossy(),);
                }

                let file_name = file.file_name().unwrap();
                let name = file_name.to_string_lossy().into_owned();
                files.push((file, name));
            }

            if parallel {
                files.par_iter().for_each(|(input_file, _file_name)| {
                    match ScriptFile::parse(input_file) {
                        Ok(script) => {
                            println!("{:#?}", script.instructions());
                        }
                        Err(e) => tracing::error!(error = ?e),
                    }
                });
            } else {
                for (input_file, _file_name) in files {
                    tracing::info!(path = ?input_file, "parse");
                    let script = ScriptFile::parse(input_file)?;
                    println!("{:#?}", script.instructions());
                }
            }
        }
        Command::Run {
            input,
            timeout,
            parallel,
            logs,
        } => {
            if let Some(logs) = logs {
                init_subscriber(logs, None)?;
            }

            let mut files = Vec::new();
            for file in input {
                if !file.exists() {
                    bail!("file {} does not exist", file.to_string_lossy(),);
                }

                let file_name = file.file_name().unwrap();
                let name = file_name.to_string_lossy().into_owned();
                files.push((file, name));
            }

            if parallel {
                files.par_iter().for_each(
                    |(input_file, file_name)| match run(
                        &input_file,
                        &file_name,
                        timeout,
                    ) {
                        Ok(_) => {}
                        Err(e) => tracing::error!(error = ?e),
                    },
                );
            } else {
                for (input_file, file_name) in files {
                    run(&input_file, &file_name, timeout)?;
                }
            }

            /*
            let scripts = ScriptFile::parse_files(input)?;
            for script in scripts {
                let file_name = script.path().file_name().unwrap();
                let mut options = InterpreterOptions::new(timeout);
                options.id = Some(file_name.to_string_lossy().into_owned());
                script.run(&options)?;
            }
            */
        }
        Command::Record {
            parallel,
            overwrite,
            output,
            input,
            timeout,
            delay,
            prompt,
            shell,
            type_pragma,
            trim_lines,
            cols,
            rows,
            deviation,
            logs,
        } => {
            if let Some(logs) = logs {
                init_subscriber(logs, None)?;
            }

            let mut files = Vec::new();
            for file in input {
                if !file.exists() {
                    bail!("file {} does not exist", file.to_string_lossy(),);
                }

                let file_name = file.file_name().unwrap();
                let name = file_name.to_string_lossy().into_owned();
                let mut output_file = output.join(&name);
                output_file.set_extension("cast");

                if !file.exists() {
                    bail!("file {} does not exist", file.to_string_lossy(),);
                }

                if !overwrite && output_file.exists() {
                    bail!(
                        "file {} already exists, use --overwrite to replace",
                        output_file.to_string_lossy(),
                    );
                }
                files.push((file, output_file, name));
            }

            let cinema = CinemaOptions {
                delay,
                prompt: prompt.clone(),
                shell: shell.clone(),
                type_pragma,
                deviation,
                cols,
                rows,
            };

            if parallel {
                files.par_iter().for_each(
                    |(input_file, output_file, file_name)| match record(
                        &input_file,
                        &output_file,
                        &file_name,
                        &cinema,
                        timeout,
                        trim_lines,
                        overwrite,
                    ) {
                        Ok(_) => {}
                        Err(e) => tracing::error!(error = ?e),
                    },
                );
            } else {
                for (input_file, output_file, file_name) in files {
                    record(
                        &input_file,
                        &output_file,
                        &file_name,
                        &cinema,
                        timeout,
                        trim_lines,
                        overwrite,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn run(input_file: &PathBuf, file_name: &str, timeout: u64) -> Result<()> {
    let script = ScriptFile::parse(input_file)?;
    let mut options = InterpreterOptions::new(timeout);
    options.id = Some(file_name.to_owned());
    script.run(&options)?;
    Ok(())
}

fn record(
    input_file: &PathBuf,
    output_file: &PathBuf,
    file_name: &str,
    cinema: &CinemaOptions,
    timeout: u64,
    trim_lines: u64,
    overwrite: bool,
) -> Result<()> {
    let script = ScriptFile::parse(input_file)?;
    let mut options = InterpreterOptions::new_recording(
        output_file.clone(),
        overwrite,
        cinema.clone(),
        timeout,
    );

    options.id = Some(file_name.to_owned());
    script.run(&options)?;

    if trim_lines > 0 {
        trim_exit(&output_file, trim_lines)?;
    }
    Ok(())
}

#[doc(hidden)]
fn init_subscriber(
    logs_dir: PathBuf,
    default_log_level: Option<String>,
) -> Result<()> {
    let logfile =
        RollingFileAppender::new(Rotation::DAILY, logs_dir, LOG_FILE_NAME);

    let default_log_level = default_log_level.unwrap_or_else(|| {
        "anticipate=debug,anticipate_core=debug".to_owned()
    });
    let env_layer = tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG").unwrap_or(default_log_level),
    );
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_file(false)
        .with_line_number(false)
        .with_target(false);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_file(false)
        .with_line_number(false)
        .with_ansi(false)
        .json()
        .with_writer(logfile);

    tracing_subscriber::registry()
        .with(env_layer)
        .with(fmt_layer)
        .with(file_layer)
        .try_init()?;

    Ok(())
}

#[doc(hidden)]
fn trim_exit(filename: impl AsRef<Path>, trim_lines: u64) -> io::Result<()> {
    let mut file = File::open(filename.as_ref())?;
    let file_size = file.seek(SeekFrom::End(0))?;
    let mut cursor = file_size;
    let mut bytes_read = 0;
    let mut num_lines = 0;

    // Read backwards and count newlines
    loop {
        if cursor > 0 {
            cursor -= 1;
            file.seek(SeekFrom::Start(cursor))?;
        } else {
            break;
        }

        let mut buf = [0; 1];
        let byte = file.read_exact(&mut buf);
        if byte.is_err() {
            break;
        }

        if &buf == b"\n" {
            num_lines += 1;
        }

        if num_lines == trim_lines + 1 {
            break;
        }

        bytes_read += 1;
    }

    // Truncate the file
    if bytes_read < file_size {
        let file = OpenOptions::new().write(true).open(filename.as_ref())?;
        let new_len = file_size - bytes_read;
        tracing::debug!(
            len = %new_len,
            file = ?filename.as_ref(),
            "truncate",
        );
        file.set_len(new_len)?;
    }

    Ok(())
}
