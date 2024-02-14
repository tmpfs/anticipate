use anticipate_core::{CinemaOptions, InterpreterOptions, ScriptFile};
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const LOG_FILE_NAME: &str = "anticipate.log";

fn main() -> Result<()> {
    if let Err(e) = start() {
        tracing::error!(error = ?e);
        std::process::exit(1);
    }
    Ok(())
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Anticipate {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Parse scripts and print the instructions.
    Parse {
        /// Directory to write logs.
        #[clap(short, long)]
        logs: Option<PathBuf>,

        /// Input file paths.
        input: Vec<PathBuf>,
    },
    /// Run scripts.
    Run {
        /// Directory to write logs.
        #[clap(short, long)]
        logs: Option<PathBuf>,

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

        /// Directory for recordings.
        output: PathBuf,

        /// Input file paths.
        input: Vec<PathBuf>,
    },
}

fn start() -> Result<()> {
    let args = Anticipate::parse();
    match args.cmd {
        Command::Parse { input, logs } => {
            if let Some(logs) = logs {
                init_subscriber(logs, None)?;
            }
            let scripts = ScriptFile::parse_files(input)?;
            for script in scripts {
                println!(
                    "{:#?}",
                    script.borrow_instructions().as_ref().unwrap()
                );
            }
        }
        Command::Run {
            input,
            timeout,
            logs,
        } => {
            if let Some(logs) = logs {
                init_subscriber(logs, None)?;
            }
            let scripts = ScriptFile::parse_files(input)?;
            for script in scripts {
                let options = InterpreterOptions::new(timeout);
                script.run(options);
            }
        }
        Command::Record {
            overwrite,
            output,
            input,
            timeout,
            delay,
            prompt,
            shell,
            type_pragma,
            trim_lines,
            deviation,
            logs,
        } => {
            if let Some(logs) = logs {
                init_subscriber(logs, None)?;
            }

            let scripts = ScriptFile::parse_files(input)?;
            for script in scripts {
                let file_name = script.borrow_path().file_name().unwrap();
                let mut output_file = output.join(file_name);
                output_file.set_extension("cast");

                if !overwrite && output_file.exists() {
                    bail!(
                        "file {} already exists, use --overwrite to replace",
                        output_file.to_string_lossy(),
                    );
                }

                let cinema = CinemaOptions {
                    delay,
                    prompt: prompt.clone(),
                    shell: shell.clone(),
                    type_pragma,
                    deviation,
                };

                let options = InterpreterOptions::new_recording(
                    output_file.clone(),
                    overwrite,
                    cinema,
                    timeout,
                );

                script.run(options);

                if trim_lines > 0 {
                    trim_exit(&output_file, trim_lines)?;
                }
            }
        }
    }
    Ok(())
}

pub fn init_subscriber(
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
