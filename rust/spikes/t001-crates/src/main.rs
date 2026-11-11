// T001 scratch: one use of each candidate crate. Default run prints facts and exits;
// `mcp` serves one tool over stdio so a handshake can be tried by hand.
use clap::{Parser, Subcommand};
use rmcp::{ServerHandler, ServiceExt, handler::server::tool::ToolRouter, tool, tool_handler, tool_router};

#[derive(Parser)]
#[command(name = "t001-crates", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Serve a one-tool MCP server on stdio.
    Mcp,
}

#[derive(Clone)]
struct Spike {
    #[expect(dead_code, reason = "tool_handler reads this field")]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Spike {
    fn new() -> Self {
        Self { tool_router: Self::tool_router() }
    }

    #[tool(description = "Return the word pong")]
    async fn ping(&self) -> String {
        "pong".to_string()
    }
}

#[tool_handler]
impl ServerHandler for Spike {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    if let Some(Cmd::Mcp) = cli.cmd {
        // stdout is the protocol channel here: print nothing else.
        let service = Spike::new().serve(rmcp::transport::stdio()).await?;
        service.waiting().await?;
        return Ok(());
    }

    // rusqlite: in-memory SQLite, print the version
    let conn = rusqlite::Connection::open_in_memory()?;
    let v: String = conn.query_row("select sqlite_version()", [], |r| r.get(0))?;
    println!("sqlite_version = {v} (rusqlite {})", rusqlite::version());

    // serde_yaml_ng: parse a small document; a bare `no` / `yes` must stay a string-or-bool by YAML 1.2 rules
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str("a: 1\nflag: no\nlist: [x, y]\n")?;
    println!("yaml flag parsed as = {:?}", doc["flag"]);

    // keyring: only ask whether a credential store exists; never writes or reads a secret
    match keyring::Entry::store_status() {
        Ok(()) => println!("keyring store = available"),
        Err(e) => println!("keyring store = unavailable: {e}"),
    }

    // axum: bind a router on 127.0.0.1 with a free port, print it, drop it
    let app = axum::Router::new().route("/", axum::routing::get(|| async { "ok" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    println!("axum would serve on {}", listener.local_addr()?);
    let server = axum::serve(listener, app);
    drop(server);
    Ok(())
}
