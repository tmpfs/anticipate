//! Script automation tool with support for recording
//! using [asciinema](https://asciinema.org/).
//!
//! For programmatic access use the [anticipate-core](https://docs.rs/anticipate-core) crate, see [the repository](https://github.com/tmpfs/anticipate/) for examples.
use anticipate_core::{CinemaOptions, InterpreterOptions, ScriptFile};
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use rayon::prelude::*;
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const TICK: &str = "✓";
const ERROR: &str = "Err";

#[doc(hidden)]
fn main() -> Result<()> {
    if let Err(e) = start() {
        fail(e);
    }
    Ok(())
}

fn fail(e: impl std::fmt::Display + std::fmt::Debug) {
    tracing::error!(error = ?e);
    error(e.to_string());
    std::process::exit(1);
}

/// Print a success message.
pub fn success(msg: impl AsRef<str>) {
    let out = format!("{} {}", msg.as_ref().green(), TICK.green());
    println!("{}", out);
}

/// Print a info message.
pub fn info(msg: impl AsRef<str>) {
    let out = format!("{}", msg.as_ref().yellow());
    println!("{}", out);
}

/// Print an error message for failure.
pub fn error(msg: impl AsRef<str>) {
    let out = format!("{} {}", ERROR.red(), msg.as_ref());
    println!("{}", out);
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
        /// Enable logging.
        #[clap(short, long, env = "ANTICIPATE_LOG", hide_env_values = true)]
        log: bool,

        /// Parse scripts in parallel.
        #[clap(short, long)]
        parallel: bool,

        /// Input file paths.
        input: Vec<PathBuf>,
    },
    /// Run scripts.
    Run {
        /// Enable logging.
        #[clap(short, long, env = "ANTICIPATE_LOG", hide_env_values = true)]
        log: bool,

        /// Scripts to run beforehand in sequence.
        #[clap(short, long)]
        setup: Vec<PathBuf>,

        /// Scripts to run afterwards in sequence.
        #[clap(short, long)]
        teardown: Vec<PathBuf>,

        /// Execute scripts in parallel.
        #[clap(short, long)]
        parallel: bool,

        /// Timeout for the pseudo-terminal.
        #[clap(short, long, default_value = "5000")]
        timeout: u64,

        /// Echo input and output.
        #[clap(short, long, env = "ANTICIPATE_ECHO", hide_env_values = true)]
        echo: bool,

        /// Format input and output logs (requires --echo).
        #[clap(
            short,
            long,
            env = "ANTICIPATE_FORMAT",
            hide_env_values = true
        )]
        format: bool,

        /// Print comments.
        #[clap(long)]
        print_comments: bool,

        /// Input file paths.
        input: Vec<PathBuf>,
    },

    /// Record using asciinema.
    #[clap(alias = "rec")]
    Record {
        /// Enable logging.
        #[clap(short, long, env = "ANTICIPATE_LOG", hide_env_values = true)]
        log: bool,

        /// Scripts to record beforehand in sequence.
        #[clap(short, long)]
        setup: Vec<PathBuf>,

        /// Scripts to record afterwards in sequence.
        #[clap(short, long)]
        teardown: Vec<PathBuf>,

        /// Execute scripts in parallel.
        #[clap(short, long)]
        parallel: bool,

        /// Timeout for the pseudo-terminal.
        #[clap(short, long, default_value = "5000")]
        timeout: u64,

        /// Echo input and output.
        #[clap(short, long, env = "ANTICIPATE_ECHO", hide_env_values = true)]
        echo: bool,

        /// Format input and output logs (requires --echo).
        #[clap(
            short,
            long,
            env = "ANTICIPATE_FORMAT",
            hide_env_values = true
        )]
        format: bool,

        /// Print comments.
        #[clap(long)]
        print_comments: bool,

        /// Overwrite existing recordings.
        #[clap(short, long)]
        overwrite: bool,

        /// Delay between keystrokes.
        #[clap(short, long, default_value = "75")]
        delay: u64,

        /// Standard deviation for gaussian distribution.
        #[clap(long, default_value = "15.0")]
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
        #[clap(long, default_value = "1")]
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
            log,
            parallel,
        } => {
            if log {
                init_subscriber()?;
            }

            let files = check_files(input)?;

            if parallel {
                files.par_iter().for_each(|(input_file, file_name)| {
                    if let Err(e) = parse(input_file, file_name) {
                        fail(e);
                    }
                });
            } else {
                for (input_file, file_name) in files {
                    parse(&input_file, &file_name)?;
                }
            }
        }
        Command::Run {
            input,
            timeout,
            parallel,
            log,
            echo,
            format,
            print_comments,
            setup,
            teardown,
        } => {
            if log {
                init_subscriber()?;
            }

            let files = check_files(input)?;
            if !setup.is_empty() {
                let files = check_files(setup)?;
                for (input_file, file_name) in files {
                    run(
                        &input_file,
                        &file_name,
                        timeout,
                        echo,
                        format,
                        print_comments,
                    )?;
                }
            }

            if parallel {
                files.par_iter().for_each(
                    |(input_file, file_name)| match run(
                        input_file,
                        file_name,
                        timeout,
                        echo,
                        format,
                        print_comments,
                    ) {
                        Ok(_) => {}
                        Err(e) => fail(e),
                    },
                );
            } else {
                for (input_file, file_name) in files {
                    run(
                        &input_file,
                        &file_name,
                        timeout,
                        echo,
                        format,
                        print_comments,
                    )?;
                }
            }

            if !teardown.is_empty() {
                let files = check_files(teardown)?;
                for (input_file, file_name) in files {
                    run(
                        &input_file,
                        &file_name,
                        timeout,
                        echo,
                        format,
                        print_comments,
                    )?;
                }
            }
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
            log,
            echo,
            format,
            print_comments,
            setup,
            teardown,
        } => {
            if log {
                init_subscriber()?;
            }

            let cinema = CinemaOptions {
                delay,
                shell: shell.clone(),
                type_pragma,
                deviation,
                cols,
                rows,
            };

            let files = check_recording_files(input, &output, overwrite)?;
            if !setup.is_empty() {
                let files = check_recording_files(setup, &output, overwrite)?;
                for (input_file, output_file, file_name) in files {
                    record(
                        &input_file,
                        &output_file,
                        &file_name,
                        &cinema,
                        timeout,
                        trim_lines,
                        overwrite,
                        echo,
                        format,
                        &prompt,
                        print_comments,
                    )?;
                }
            }

            if parallel {
                files.par_iter().for_each(
                    |(input_file, output_file, file_name)| match record(
                        input_file,
                        output_file,
                        file_name,
                        &cinema,
                        timeout,
                        trim_lines,
                        overwrite,
                        echo,
                        format,
                        &prompt,
                        print_comments,
                    ) {
                        Ok(_) => {}
                        Err(e) => fail(e),
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
                        echo,
                        format,
                        &prompt,
                        print_comments,
                    )?;
                }
            }

            if !teardown.is_empty() {
                let files =
                    check_recording_files(teardown, &output, overwrite)?;
                for (input_file, output_file, file_name) in files {
                    record(
                        &input_file,
                        &output_file,
                        &file_name,
                        &cinema,
                        timeout,
                        trim_lines,
                        overwrite,
                        echo,
                        format,
                        &prompt,
                        print_comments,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn parse(input_file: &PathBuf, file_name: &str) -> Result<()> {
    tracing::debug!(path = ?input_file, "parse");

    info(format!("Parse {}", file_name));
    match ScriptFile::parse(input_file) {
        Ok(script) => {
            println!("{:#?}", script.instructions());
        }
        Err(e) => fail(e),
    }
    success(format!("   Ok {}", file_name));
    Ok(())
}

fn run(
    input_file: &PathBuf,
    file_name: &str,
    timeout: u64,
    echo: bool,
    format: bool,
    print_comments: bool,
) -> Result<()> {
    info(format!("Run {}", file_name));
    let script = ScriptFile::parse(input_file)?;
    let mut options =
        InterpreterOptions::new(timeout, echo, format, print_comments);
    options.id = Some(file_name.to_owned());
    script.run(options)?;
    success(format!(" Ok {}", file_name));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn record(
    input_file: &PathBuf,
    output_file: &PathBuf,
    file_name: &str,
    cinema: &CinemaOptions,
    timeout: u64,
    trim_lines: u64,
    overwrite: bool,
    echo: bool,
    format: bool,
    prompt: &str,
    print_comments: bool,
) -> Result<()> {
    info(format!("Rec {}", file_name));
    let script = ScriptFile::parse(input_file)?;
    let mut options = InterpreterOptions::new_recording(
        output_file.clone(),
        overwrite,
        cinema.clone(),
        timeout,
        echo,
        format,
        print_comments,
    );

    options.prompt = Some(prompt.to_string());
    options.id = Some(file_name.to_owned());
    script.run(options)?;

    if trim_lines > 0 {
        trim_exit(output_file, trim_lines)?;
    }
    success(format!(" Ok {}", file_name));
    Ok(())
}

#[doc(hidden)]
fn init_subscriber() -> Result<()> {
    let default_log_level =
        "anticipate=debug,anticipate_core=debug".to_owned();
    let env_layer = tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG").unwrap_or(default_log_level),
    );
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_file(false)
        .with_line_number(false)
        .with_target(false);

    tracing_subscriber::registry()
        .with(env_layer)
        .with(fmt_layer)
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

fn check_files(input: Vec<PathBuf>) -> Result<Vec<(PathBuf, String)>> {
    let mut files = Vec::new();
    for file in input {
        if !file.exists() {
            bail!("file {} does not exist", file.to_string_lossy());
        }

        let file_name = file.file_name().unwrap();
        let name = file_name.to_string_lossy().into_owned();
        files.push((file, name));
    }
    Ok(files)
}

fn check_recording_files(
    input: Vec<PathBuf>,
    output: &Path,
    overwrite: bool,
) -> Result<Vec<(PathBuf, PathBuf, String)>> {
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
    Ok(files)
}
