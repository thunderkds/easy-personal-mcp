//! T044: `GET /api/dirs`, `GET /api/jobs` and the MCP tool `kit_findings` over the real binary. `dirs` replays the cases of
//! `rust/parity/golden-jobs/dirs.json` (Python's own `list_dirs`), `jobs` runs fakes (a `/bin/sh` script on a scratch `PATH`;
//! the binary's own `PATH` is that folder alone) and compares what does not depend on the clock to `jobs.json`, and
//! `kit_findings` equals the `golden-rich` file but for `as_of` (the binary reads the real clock; the frozen-clock replay is
//! `crates/core/tests/kitperf_parity.rs`). Temp folders only: the owner's data folder, the real `systemctl`, `crontab`,
//! `claude` and keychain are never reached.

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

fn parity(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

fn read_json(rel: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(parity(rel)).unwrap()).unwrap()
}

fn temp(tag: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "easy-personal-rs-t044-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::canonicalize(&dir).unwrap()
}

fn is_root() -> bool {
    let dir = temp("uid");
    let root = fs::metadata(&dir).unwrap().uid() == 0;
    let _ = fs::remove_dir_all(&dir);
    root
}

fn build_tree(root: &Path) {
    let tree = read_json("golden-jobs/tree.json");
    for entry in tree["entries"].as_array().unwrap() {
        let (kind, rel, target) = (
            entry[0].as_str().unwrap(),
            entry[1].as_str().unwrap(),
            &entry[2],
        );
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        match kind {
            "dir" => fs::create_dir_all(&path).unwrap(),
            "file" => fs::write(&path, "x").unwrap(),
            _ => symlink(target.as_str().unwrap(), &path).unwrap(),
        }
    }
    for i in 0..tree["many"].as_u64().unwrap() {
        fs::create_dir_all(root.join("many").join(format!("d{i:03}"))).unwrap();
    }
    fs::create_dir_all(root.join(tree["many_repo"].as_str().unwrap()).join(".git")).unwrap();
}

fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// A `/bin/sh` fake that answers each recorded call of `calls` with its recorded stdout and exit code.
fn install_fake(bin: &Path, tool: &str, calls: &[&Value]) {
    let mut script = String::from("#!/bin/sh\ncase \"$*\" in\n");
    for (i, call) in calls.iter().enumerate() {
        let argv: Vec<&str> = call["argv"].as_array().unwrap()[1..]
            .iter()
            .map(|a| a.as_str().unwrap())
            .collect();
        let out = bin.parent().unwrap().join(format!("{tool}-{i}.out"));
        fs::write(&out, call["stdout"].as_str().unwrap()).unwrap();
        script += &format!(
            "  {}) while IFS= read -r l || [ -n \"$l\" ]; do printf '%s\\n' \"$l\"; done < {}; exit {};;\n",
            quote(&argv.join(" ")),
            quote(out.to_str().unwrap()),
            call["exit"]
        );
    }
    script += "esac\nexit 3\n";
    let path = bin.join(tool);
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

struct Response {
    status: u16,
    body: String,
}

impl Response {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_else(|_| panic!("not JSON: {:?}", self.body))
    }
}

struct Server {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
    base: PathBuf,
}

impl Server {
    /// `serve` with an empty environment but `HOME`, `EASY_PERSONAL_HOME`, `TZ=UTC` and `PATH` = `bin` (a folder of fakes), run in `cwd`.
    fn start(base: PathBuf, bin: &Path, home: &Path, cwd: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .arg("serve")
            .env_clear()
            .env("HOME", home)
            .env("EASY_PERSONAL_HOME", base.join("data"))
            .env("TZ", "UTC")
            .env("PATH", bin)
            .current_dir(cwd)
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
            .unwrap_or_else(|| panic!("first line {line:?}"));
        let (port, token) = url.split_once("/#").expect("URL fragment");
        Server {
            child,
            _stdout: stdout,
            port: port.parse().unwrap(),
            token: token.to_string(),
            base,
        }
    }

    fn raw(&self, request: &str) -> Response {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((text.as_str(), ""));
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("no status in {text:?}"));
        Response {
            status,
            body: body.to_string(),
        }
    }

    fn send(&self, method: &str, path: &str, token: Option<&str>, host: Option<&str>) -> Response {
        let host = host.map_or_else(|| format!("127.0.0.1:{}", self.port), str::to_string);
        let mut req = format!("{method} {path} HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\n");
        if let Some(token) = token {
            req += &format!("x-dashboard-token: {token}\r\n");
        }
        self.raw(&(req + "Connection: close\r\n\r\n"))
    }

    fn get(&self, path: &str) -> Response {
        self.send("GET", path, Some(&self.token), None)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.base);
    }
}

/// A scratch base folder holding `bin/`, `data/` and `tree/` (the folder tree of `tree.json`), and a server over it.
fn launch(fakes: &str) -> (Server, PathBuf) {
    let base = temp("srv");
    let (bin, tree) = (base.join("bin"), base.join("tree"));
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(base.join("data")).unwrap();
    build_tree(&tree);
    if !fakes.is_empty() {
        let golden = read_json("golden-jobs/jobs.json");
        let case = golden["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["case"] == fakes)
            .unwrap();
        for tool in ["systemctl", "crontab"] {
            if case["tools"][tool] == true {
                let calls: Vec<&Value> = case["calls"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|c| c["argv"][0] == tool)
                    .collect();
                install_fake(&bin, tool, &calls);
            }
        }
    }
    let server = Server::start(base.clone(), &bin, &tree.join("home"), &tree.join("main"));
    (server, tree)
}

#[test]
fn dirs_equals_python_over_http_for_every_case() {
    let (server, tree) = launch("");
    let root = tree.to_str().unwrap().to_string();
    let locked = tree.join("main/locked");
    let (mut equal, mut total) = (0, 0);
    for case in read_json("golden-jobs/dirs.json")["cases"]
        .as_array()
        .unwrap()
    {
        let name = case["case"].as_str().unwrap();
        if case["unprivileged"] == true && is_root() {
            continue;
        }
        let query = match case["path"].as_str() {
            None => String::new(), // no `path`: the server's working folder, `<tree>/main`
            Some(template) => format!("?path={}", encode(&template.replace("<ROOT>", &root))),
        };
        if case["unprivileged"] == true {
            fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        }
        let got = server.get(&format!("/api/dirs{query}"));
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        total += 1;
        assert_eq!(got.status, case["status"], "{name}");
        let want: Value =
            serde_json::from_str(&case["body"].to_string().replace("<ROOT>", &root)).unwrap();
        if want.get("error") == Some(&Value::Null) {
            assert!(got.json()["error"].is_string(), "{name}"); // CPython's own text: status and the key only
        } else {
            assert_eq!(got.json(), want, "{name}");
        }
        equal += 1;
    }
    println!("dirs over HTTP equal/total: {equal}/{total}");
    assert_eq!(total, 14 - usize::from(is_root()));
}

#[test]
fn dirs_and_jobs_obey_the_guard_and_refuse_write_methods() {
    let (server, _tree) = launch("no_tools");
    for route in ["/api/dirs", "/api/jobs"] {
        assert_eq!(
            server.send("GET", route, None, None).status,
            403,
            "{route} no token"
        );
        assert_eq!(
            server.send("GET", route, Some("wrong"), None).status,
            403,
            "{route} wrong token"
        );
        assert_eq!(
            server
                .send("GET", route, Some(&server.token), Some("localhost:1"))
                .status,
            403,
            "{route} bad host"
        );
        for method in ["POST", "PUT", "PATCH", "DELETE"] {
            assert_eq!(
                server.send(method, route, Some(&server.token), None).status,
                405,
                "{route} {method}"
            );
        }
        assert_eq!(server.get(route).status, 200, "{route}");
    }
    // a 4,000-character path and an unreadable-looking one answer without echoing it
    let long = server.get(&format!("/api/dirs?path=/{}", "a".repeat(4000)));
    assert_eq!(
        (long.status, long.json()),
        (400, json!({"error": "Not an existing folder"}))
    );
    assert!(!long.body.contains("aaaa"));
}

#[test]
fn jobs_lists_what_the_fakes_say_and_nothing_else() {
    let (server, tree) = launch("mixed");
    let got = server.get("/api/jobs");
    assert_eq!(got.status, 200);
    let body = got.json();
    let golden = read_json("golden-jobs/jobs.json");
    let want = &golden["cases"][0]["body"];
    assert_eq!(body["sources"], want["sources"]);
    assert_eq!(body["hidden"], want["hidden"]);
    let rows = |v: &Value, kind: &str| -> Vec<Value> {
        v["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|j| j["kind"] == kind)
            .cloned()
            .map(|mut j| {
                if kind == "cron" {
                    j.as_object_mut().unwrap().remove("next_run"); // counted from the real clock here
                }
                j
            })
            .collect()
    };
    assert_eq!(rows(&body, "systemd"), rows(want, "systemd"));
    assert_eq!(rows(&body, "cron"), rows(want, "cron"));
    for secret in ["SECRET", "--token", "/usr/bin", "FOO=", "hello there"] {
        assert!(!got.body.contains(secret), "{secret}");
    }
    let _ = tree;
}

/// T052: a crontab step no 64-bit integer can add twice (`1/9223372036854775807`) used to panic the request thread (debug)
/// or wrap (release); the route answers 200 with every line's row, as Python's set of `range(first, last + 1, step)`.
#[test]
fn crontab_steps_that_overflow_still_answer_200_with_every_row() {
    let (server, _tree) = launch("cron_steps");
    let got = server.get("/api/jobs");
    assert_eq!(got.status, 200);
    let body = got.json();
    let golden = read_json("golden-jobs/jobs.json");
    let case = golden["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["case"] == "cron_steps")
        .unwrap();
    let schedules = |v: &Value| -> Vec<(Value, bool)> {
        v["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|j| (j["schedule"].clone(), j["next_run"].is_null())) // next_run is counted from the real clock here
            .collect()
    };
    assert_eq!(schedules(&body), schedules(&case["body"]));
    assert_eq!(body["jobs"].as_array().unwrap().len(), 12);
    assert_eq!(body["sources"], case["body"]["sources"]);
}

#[test]
fn with_an_empty_path_both_kinds_are_no_data_and_the_route_still_answers_200() {
    let (server, _tree) = launch("");
    let got = server.get("/api/jobs");
    assert_eq!(got.status, 200);
    assert_eq!(
        got.json(),
        json!({"jobs": [], "hidden": 0, "sources": {"systemd": "no data", "cron": "no data"}})
    );
}

fn mcp_session(home: &Path, calls: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
        .arg("mcp")
        .env_clear()
        .env("HOME", home)
        .env("EASY_PERSONAL_HOME", home)
        .env("TZ", "UTC")
        .env("PATH", home.join("none"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let init = json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}});
    writeln!(stdin, "{init}").unwrap();
    for (i, call) in calls.iter().enumerate() {
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc":"2.0","id":i + 1,"method":call["method"],"params":call["params"]})
        )
        .unwrap();
    }
    drop(stdin);
    let mut out = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    let _ = child.wait();
    let mut answers: Vec<Value> = out
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .filter(|v: &Value| v["id"] != 0)
        .collect();
    answers.sort_by_key(|v| v["id"].as_u64()); // the server may answer out of order
    answers
}

#[test]
fn kit_findings_over_stdio_equals_the_rich_golden_and_checks_its_arguments() {
    let home = temp("mcp");
    fs::copy(
        parity("store-rich/store-v11.sqlite"),
        home.join("store.sqlite"),
    )
    .unwrap();
    let call = |args: Value| json!({"method": "tools/call", "params": {"name": "kit_findings", "arguments": args}});
    let answers = mcp_session(
        &home,
        &[
            call(json!({})),
            call(json!({"limit": 10})),
            call(json!({"since": "2026-01-06T00:00:00Z"})),
            call(json!({"limit": 0})),
            call(json!({"limit": "x"})),
            call(json!({"limit": null})),
            call(json!({"since": "garbage"})),
            call(json!({"since": 5})),
            call(json!({"limit": 9_999_999_999_999_999_999_u64})),
            json!({"method": "tools/list", "params": {}}),
        ],
    );
    let structured = |a: &Value| {
        let mut v = a["result"]["structuredContent"].clone();
        v.as_object_mut().map(|o| o.remove("as_of"));
        v
    };
    let strip = |name: &str| {
        let mut v = read_json(&format!("golden-rich/{name}.json"));
        v.as_object_mut().unwrap().remove("as_of");
        v
    };
    assert_eq!(structured(&answers[0]), strip("mcp__kit_findings__all"));
    assert_eq!(
        structured(&answers[1]),
        strip("mcp__kit_findings__limit_10")
    );
    assert_eq!(
        structured(&answers[2]),
        strip("mcp__kit_findings__since_week")
    );
    assert_eq!(
        structured(&answers[3])["findings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        structured(&answers[8])["findings"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for bad in [4, 5, 6, 7] {
        assert_eq!(answers[bad]["result"]["isError"], true, "{bad}");
        let text = answers[bad]["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        assert!(
            text.starts_with("Error executing tool kit_findings: "),
            "{text}"
        );
        assert!(!text.contains(home.to_str().unwrap()));
    }
    assert_eq!(
        answers[6]["result"]["content"][0]["text"],
        "Error executing tool kit_findings: Invalid isoformat string: 'garbage'"
    );
    let listed = answers[9]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "kit_findings")
        .unwrap()
        .clone();
    let golden = read_json("golden/mcp__tools_list.json");
    let want = golden
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "kit_findings")
        .unwrap();
    for key in ["description", "inputSchema", "outputSchema"] {
        assert_eq!(listed[key], want[key], "{key}");
    }
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn kit_findings_with_no_store_is_a_tool_error_without_a_path() {
    let home = temp("nostore");
    let answers = mcp_session(
        &home,
        &[json!({"method": "tools/call", "params": {"name": "kit_findings", "arguments": {}}})],
    );
    assert_eq!(answers[0]["result"]["isError"], true);
    assert!(!answers[0].to_string().contains(home.to_str().unwrap()));
    let _ = fs::remove_dir_all(&home);
}
