use clap::ArgAction;
use clap::{Args, Parser, Subcommand};
use dotenvy::dotenv;
use kueaplan_server::cli_error::CliError;
use log::{error, info, warn};
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

    info!(
        "This is the online kueaplan server v{}",
        kueaplan_server::get_version()
    );
    let result = run_main_command(args.command);
    if let Err(err) = result {
        error!("{}", err);
        std::process::exit(err.exit_code());
    }
}

fn run_main_command(command: Command) -> Result<(), CliError> {
    match command {
        Command::Event(EventCommand::List) => {
            kueaplan_server::cli::manage_events::print_event_list()?;
        }
        Command::Event(EventCommand::Import { path }) => {
            kueaplan_server::cli::file_io::load_event_from_file(&path)?;
        }
        Command::Event(EventCommand::Export { event_id, path }) => {
            kueaplan_server::cli::file_io::export_event_to_file(event_id, &path)?;
        }
        Command::Serve => {
            kueaplan_server::cli::database_migration::check_migration_state()?;
            kueaplan_server::web::serve()?;
        }
        Command::MigrateDatabase => {
            kueaplan_server::cli::database_migration::run_migrations()?;
        }
    }
    Ok(())
}

/// Online KüA-Plan HTTP server and commandline management tool
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
    MigrateDatabase,
    /// Serve the KüA-Plan web application
    Serve,
    /// Collection of sub commands for managing Events
    /// Execute all pending database migrations to run this version of the kueaplan
    #[clap(subcommand)]
    Event(EventCommand),
}

#[derive(Debug, Subcommand)]
enum EventCommand {
    /// List all events in the database
    List,
    /// Load event data (except for passphrases) from JSON file
    Import {
        /// The path of the JSON file to read from
        path: PathBuf,
    },
    /// Export full event (except for passphrases) to JSON file
    Export {
        /// The id of the event to be exported
        event_id: i32,
        /// The path of the JSON file to read from
        path: PathBuf,
    },
}

#[derive(Debug, Args)]
struct GlobalOpts {
    /// Verbosity level (can be specified multiple times)
    #[clap(long, short, global = true, action = ArgAction::Count)]
    verbose: u8,
}
