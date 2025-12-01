use clap::ArgAction;
use clap::{Args, Parser, Subcommand};
use dotenvy::dotenv;
use kueaplan_server::cli::EventIdOrSlug;
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
        Command::Event(EventCommand::Import { path, keep_uuids }) => {
            kueaplan_server::cli::file_io::load_event_from_file(&path, !keep_uuids)?;
        }
        Command::Event(EventCommand::Export {
            event_id_or_slug,
            path,
        }) => {
            kueaplan_server::cli::file_io::export_event_to_file(event_id_or_slug, &path)?;
        }
        Command::Event(EventCommand::Create) => {
            kueaplan_server::cli::manage_events::create_event()?;
        }
        Command::Event(EventCommand::Delete { event_id_or_slug }) => {
            kueaplan_server::cli::manage_events::delete_event(event_id_or_slug)?;
        }
        Command::Passphrase(PassphraseCommand::List { event_id_or_slug }) => {
            kueaplan_server::cli::manage_passphrases::print_passphrase_list(event_id_or_slug)?;
        }
        Command::Passphrase(PassphraseCommand::Create { event_id_or_slug }) => {
            kueaplan_server::cli::manage_passphrases::add_passphrase(event_id_or_slug)?;
        }
        Command::Passphrase(PassphraseCommand::Edit {
            event_id_or_slug,
            passphrase_id,
        }) => {
            kueaplan_server::cli::manage_passphrases::edit_passphrase(
                event_id_or_slug,
                passphrase_id,
            )?;
        }
        Command::Passphrase(PassphraseCommand::Delete {
            event_id_or_slug,
            passphrase_id,
        }) => {
            kueaplan_server::cli::manage_passphrases::delete_passphrase(
                event_id_or_slug,
                passphrase_id,
            )?;
        }
        Command::Passphrase(PassphraseCommand::Invalidate {
            event_id_or_slug,
            passphrase_id,
        }) => {
            kueaplan_server::cli::manage_passphrases::invalidate_passphrase(
                event_id_or_slug,
                passphrase_id,
            )?;
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
#[clap(name = "kueaplan_server", version)]
pub struct CliArgs {
    #[clap(flatten)]
    global_opts: GlobalOpts,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Execute all pending database migrations to run this version of the kueaplan
    MigrateDatabase,
    /// Serve the KüA-Plan web application
    Serve,
    /// Collection of sub commands for managing Events
    #[clap(subcommand)]
    Event(EventCommand),
    /// Collection of sub commands for managing Passphrases of events
    #[clap(subcommand)]
    Passphrase(PassphraseCommand),
}

#[derive(Debug, Subcommand)]
enum EventCommand {
    /// List all events in the database
    List,
    /// Load event data (except for passphrases) from JSON file
    Import {
        /// The path of the JSON file to read from
        path: PathBuf,
        /// Keep the entries', previous dates', rooms', categories' and announcements' UUIDs,
        /// instead of generating new ones. This may cause conflicts with existing data, when the
        /// file has been exported from this server's database or when it is imported multiple
        /// times.
        #[clap(long)]
        keep_uuids: bool,
    },
    /// Export full event (except for passphrases) to JSON file
    Export {
        /// The id or slug of the event to be exported
        event_id_or_slug: EventIdOrSlug,
        /// The path of the JSON file to read from
        path: PathBuf,
    },
    /// Create a new event. Basic event data is queried interactively in the terminal.
    Create,
    /// Delete an event with all associated data.
    Delete {
        /// The id or slug of the event to be deleted
        event_id_or_slug: EventIdOrSlug,
    },
}

#[derive(Debug, Subcommand)]
enum PassphraseCommand {
    /// List all passphrases of the given event (by event id or event slug)
    List {
        /// The id or slug of the event
        event_id_or_slug: EventIdOrSlug,
    },
    /// Create a new passphrase for the given event (by event id or event slug)
    Create {
        /// The id or slug of the event
        event_id_or_slug: EventIdOrSlug,
    },
    /// Change comment or validity of the passphrase with given id from the given event (by event id
    /// or event slug)
    Edit {
        /// The id or slug of the event
        event_id_or_slug: EventIdOrSlug,
        /// The id of the passphrase to be edited
        passphrase_id: i32,
    },
    /// Delete the passphrase with given id from the given event (by event id or event slug)
    Delete {
        /// The id or slug of the event
        event_id_or_slug: EventIdOrSlug,
        /// The id of the passphrase to be deleted
        passphrase_id: i32,
    },
    /// Invalidate the passphrase with given id from the given event, i.e. set its valid_until
    /// timestamp to the current time.
    Invalidate {
        /// The id or slug of the event
        event_id_or_slug: EventIdOrSlug,
        /// The id of the passphrase to be invalidated
        passphrase_id: i32,
    },
}

#[derive(Debug, Args)]
struct GlobalOpts {
    /// Verbosity level (can be specified multiple times)
    #[clap(long, short, global = true, action = ArgAction::Count)]
    verbose: u8,
}
