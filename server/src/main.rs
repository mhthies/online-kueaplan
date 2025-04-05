use clap::ArgAction;
use clap::{Args, Parser, Subcommand};
use dotenvy::dotenv;
use log::warn;
use std::path::PathBuf;

fn main() {
    let args = CliArgs::parse();
    let dotenv_result = dotenv();

    let env = env_logger::Env::new().filter_or(
        "RUST_LOG",
        match args.global_opts.verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        },
    );
    env_logger::Builder::from_env(env).init();
    if dotenv_result.is_err() {
        warn!("Could not read .env file: {}", dotenv_result.unwrap_err());
    }

    match args.command {
        Command::LoadData { path } => kueaplan_server::file_io::load_event_from_file(
            &path,
            kueaplan_server::CliAuthTokenKey::new(),
        )
        .unwrap(),
        Command::Serve => kueaplan_server::web::serve().unwrap(),
    }
}

/// Here's my app!
#[derive(Debug, Parser)]
#[clap(name = "my-app", version)]
pub struct CliArgs {
    #[clap(flatten)]
    global_opts: GlobalOpts,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Load data from JSON file
    LoadData {
        /// The path of the JSON file to read from
        path: PathBuf,
    },
    /// Serve the KüA-Plan web application
    Serve,
}

#[derive(Debug, Args)]
struct GlobalOpts {
    /// Verbosity level (can be specified multiple times)
    #[clap(long, short, global = true, action = ArgAction::Count)]
    verbose: u8,
}
