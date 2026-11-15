//! T072 (FR-095): the real binary. `machine` and `machine --json` print the sample; `serve` answers `GET /api/machine` with a
//! first sample (the sampler takes one at start) and refuses a request with no token. `HOME` and `EASY_PERSONAL_HOME`
//! point into a temp folder (no store in it), so the owner's `~/.easy-personal` is never reached; nothing is created there.

use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const KEYS: [&str; 10] = [
    "t",
    "cpu_pct",
    "ram_used",
    "ram_total",
    "disk_used",
    "disk_total",
    "load1",
    "battery_pct",
    "charging",
    "claude_procs",
];

fn temp_home(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t072-{tag}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::canonicalize(dir).unwrap()
}

fn command(home: &PathBuf, args: &[&str]) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"));
    c.args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", home)
        .env("EASY_PERSONAL_HOME", home);
    c
}

#[test]
fn machine_text_has_one_line_per_measure() {
    let home = temp_home("text");
    let out = command(&home, &["machine"]).output().unwrap();
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    let labels: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_once(": ").map(|p| p.0))
        .collect();
    assert_eq!(
        labels,
        [
            "CPU",
            "RAM",
            "Disk",
            "Load (1 min)",
            "Battery",
            "Claude processes"
        ],
        "{text}"
    );
    #[cfg(target_os = "linux")]
    {
        // this host is Linux: RAM and load are readable, and the temp folder's disk too
        let line = |label: &str| {
            text.lines()
                .find(|l| l.starts_with(label))
                .unwrap()
                .to_string()
        };
        for label in ["RAM", "Disk", "Load"] {
            assert!(!line(label).ends_with("no data"), "{text}");
        }
    }
    assert!(
        fs::read_dir(&home).unwrap().next().is_none(),
        "the command created something in the data folder"
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn machine_json_is_one_sample_with_every_key() {
    let home = temp_home("json");
    let out = command(&home, &["machine", "--json"]).output().unwrap();
    assert!(out.status.success(), "{out:?}");
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    let object = value.as_object().unwrap();
    assert_eq!(object.len(), KEYS.len(), "{value}");
    for key in KEYS {
        assert!(object.contains_key(key), "{key} missing: {value}");
    }
    assert!(value["t"].as_i64().unwrap() > 1_700_000_000);
    let _ = fs::remove_dir_all(&home);
}

struct Server {
    child: Child,
    _stdout: BufReader<std::process::ChildStdout>,
    port: u16,
    token: String,
}

impl Server {
    fn start(home: &PathBuf) -> Server {
        let mut child = command(home, &["serve", "--no-ingest"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        let url = line
            .trim_end()
            .strip_prefix("dashboard: http://127.0.0.1:")
            .unwrap()
            .to_string();
        let (port, token) = url.split_once("/#").unwrap();
        Server {
            child,
            _stdout: stdout,
            port: port.parse().unwrap(),
            token: token.to_string(),
        }
    }

    fn get(&self, token: bool) -> (u16, Value) {
        let mut request = format!(
            "GET /api/machine HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n",
            self.port
        );
        if token {
            request += &format!("x-dashboard-token: {}\r\n", self.token);
        }
        request += "Connection: close\r\n\r\n";
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        let at = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&bytes[..at]).into_owned();
        let status = head.split(' ').nth(1).unwrap().parse().unwrap();
        (
            status,
            serde_json::from_slice(&bytes[at + 4..]).unwrap_or(Value::Null),
        )
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn serve_answers_machine_with_a_first_sample_and_guards_it() {
    let home = temp_home("serve");
    let server = Server::start(&home);
    let (status, body) = server.get(true);
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["interval_s"], 60);
    let samples = body["samples"].as_array().unwrap();
    assert_eq!(samples.len(), 1, "the sampler takes one at start: {body}");
    assert_eq!(body["latest"], samples[0]);
    for key in KEYS {
        assert!(body["latest"].get(key).is_some(), "{key} missing: {body}");
    }
    assert!(
        body["latest"]["cpu_pct"].is_null(),
        "the first sample has no CPU delta"
    );
    let (status, _) = server.get(false);
    assert_eq!(status, 403);
    assert!(
        fs::read_dir(&home).unwrap().next().is_none(),
        "serve wrote into the data folder"
    );
    drop(server);
    let _ = fs::remove_dir_all(&home);
}
