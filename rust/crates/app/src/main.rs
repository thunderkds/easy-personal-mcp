use clap::{Parser, Subcommand};
use easy_personal_core::store::{read_info, store_path};
use std::path::PathBuf;
use std::process::ExitCode;

mod api;
#[cfg(any(feature = "window", test))]
mod display;
mod ingest_cmd;
mod live_feed;
mod mcp;
mod mcp_http;
mod schedule_cmd;
mod server;
mod static_files;
mod store_check_cmd;
mod window;
mod writes;

#[derive(Parser)]
#[command(version, about = "Rust port of easy-personal")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show the store's version and row counts (opens it read-only; never writes)
    StoreInfo {
        /// Print one JSON object instead of text
        #[arg(long)]
        json: bool,
    },
    /// Read-only health check of the store: file, version, integrity, row counts, sources, backups (never writes). Exit 0 healthy, 1 a problem, 3 not fully checked, 130 Ctrl-C
    StoreCheck {
        /// Print one JSON object instead of text
        #[arg(long)]
        json: bool,
        /// Run the full integrity check with no time limit (can take minutes)
        #[arg(long)]
        full: bool,
    },
    /// Copy new Claude Code transcript metadata into the store (events only; takes the ingest lock; never migrates)
    Ingest {
        /// Transcript root of the default account (default: its own `projects` folder)
        #[arg(long, value_name = "PATH")]
        source: Option<PathBuf>,
        /// Delete events older than N days (default: keep all)
        #[arg(long, value_name = "N", allow_negative_numbers = true, value_parser = clap::value_parser!(i64).range(1..))]
        retention_days: Option<i64>,
    },
    /// Run ingest daily on a systemd user timer (same unit names as `easy-personal schedule`)
    Schedule {
        #[command(subcommand)]
        action: schedule_cmd::Action,
    },
    /// Serve the dashboard API on 127.0.0.1 (a free port, a per-run token) until Ctrl-C
    Serve {
        /// Also serve the built dashboard's files from this folder, from the same origin (no token for the page)
        #[arg(long, value_name = "DIR")]
        static_dir: Option<PathBuf>,
        /// Do not ingest before the start (default: ingest first, as Python's `dashboard`)
        #[arg(long)]
        no_ingest: bool,
    },
    /// Serve the MCP tools over stdio (JSON-RPC on stdout, nothing else) or over loopback HTTP (`--transport http`)
    Mcp(mcp::Args),
    /// Open the dashboard in a window (needs a build with `--features window` and a display)
    App {
        /// Serve the built dashboard's files from this folder, from the same origin as the API
        #[arg(long, value_name = "DIR")]
        static_dir: PathBuf,
        /// Exit 0 with `self-test: ok` once the page made one authenticated API request, exit 3 after 20 s without
        #[arg(long, hide = true)]
        self_test: bool,
        /// Do not ingest before the start (default: ingest first)
        #[arg(long)]
        no_ingest: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::StoreInfo { json } => match store_path().and_then(|path| read_info(&path)) {
            Ok(info) => {
                if json {
                    println!("{}", info.json());
                } else {
                    print!("{}", info.text());
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("easy-personal-rs: {e}");
                ExitCode::from(e.exit_code() as u8)
            }
        },
        Command::StoreCheck { json, full } => store_check_cmd::run(json, full),
        Command::Ingest {
            source,
            retention_days,
        } => ingest_cmd::run(source, retention_days),
        Command::Schedule { action } => schedule_cmd::run(action),
        Command::Serve {
            static_dir,
            no_ingest,
        } => {
            if !no_ingest {
                ingest_cmd::startup(); // before the live feed's thread starts (it starts inside `server::run`)
            }
            match server::run(static_dir.as_deref()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("easy-personal-rs: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::App {
            static_dir,
            self_test,
            no_ingest,
        } => {
            if !no_ingest && cfg!(feature = "window") {
                ingest_cmd::startup(); // a build with no window starts nothing, so it ingests nothing
            }
            window::run(&static_dir, self_test)
        }
        Command::Mcp(args) => mcp::main(args),
    }
}
