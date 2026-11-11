use clap::{Parser, Subcommand};
use easy_personal_core::store::{read_info, store_path};
use std::path::PathBuf;
use std::process::ExitCode;

mod api;
#[cfg(any(feature = "window", test))]
mod display;
mod mcp;
mod server;
mod static_files;
mod window;

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
    /// Serve the dashboard API on 127.0.0.1 (a free port, a per-run token) until Ctrl-C
    Serve {
        /// Also serve the built dashboard's files from this folder, from the same origin (no token for the page)
        #[arg(long, value_name = "DIR")]
        static_dir: Option<PathBuf>,
    },
    /// Serve the MCP tools over stdio (JSON-RPC on stdout, nothing else)
    Mcp,
    /// Open the dashboard in a window (needs a build with `--features window` and a display)
    App {
        /// Serve the built dashboard's files from this folder, from the same origin as the API
        #[arg(long, value_name = "DIR")]
        static_dir: PathBuf,
        /// Exit 0 with `self-test: ok` once the page made one authenticated API request, exit 3 after 20 s without
        #[arg(long, hide = true)]
        self_test: bool,
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
        Command::Serve { static_dir } => match server::run(static_dir.as_deref()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("easy-personal-rs: {e}");
                ExitCode::FAILURE
            }
        },
        Command::App {
            static_dir,
            self_test,
        } => window::run(&static_dir, self_test),
        Command::Mcp => match mcp::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("easy-personal-rs: {e}");
                ExitCode::FAILURE
            }
        },
    }
}
