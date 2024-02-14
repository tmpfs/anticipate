use anticipate_core::{CompileOptions, ScriptFile};
use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

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
    /// Run a single script.
    Run {
        /// Directory to write logs.
        #[clap(short, long)]
        logs: Option<PathBuf>,

        /// Directory for recordings.
        #[clap(short, long)]
        record: Option<PathBuf>,

        /// Overwrite existing recordings.
        #[clap(short, long)]
        overwrite: bool,
        
        /// Input file paths.
        paths: Vec<PathBuf>,
    },
}

fn start() -> Result<()> {
    let args = Anticipate::parse();
    match args.cmd {
        Command::Run { record, overwrite, paths, logs } => {
            if let Some(logs) = logs {
                init_subscriber(logs, None)?;
            }

            let scripts = ScriptFile::parse_files(paths)?;
            for script in scripts {
                //println!("{:#?}", script.borrow_instructions());
                let options = if let Some(record) = &record {
                    let file_name = script.borrow_path().file_name().unwrap();
                    let mut output_file = record.join(file_name);
                    output_file.set_extension("cast");
                    
                    if !overwrite && output_file.exists() {
                        bail!("file {} already exists, use --overwrite to replace", output_file.to_string_lossy());
                    }

                    CompileOptions::new_recording(output_file, overwrite)
                } else {
                    Default::default()
                };
                script.run(options);
            }
        }
    }
    Ok(())
}

pub fn init_subscriber(logs_dir: PathBuf, default_log_level: Option<String>) -> Result<()> {
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
