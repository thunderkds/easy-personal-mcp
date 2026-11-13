//! T045: `GET /api/kit/report-files` and `GET /api/kit/report-file` over the real `serve` binary: the nine rich goldens
//! (status, the four headers Python sets, the CSP the server adds, body), the guard (token, Host, Origin: 403), the write
//! methods (405), `HEAD`, repeated and missing query parameters, percent-encoded and double-encoded `..`, an oversize file
//! (413) and the fixed error bodies. The attack table of the core (symlinks, FIFO, race ...) is `crates/core/tests/
//! report_files_parity.rs`. The binary runs under `TZ=UTC` over temp copies of the rich store and `fixtures-rich/kit`;
//! `HOME`, `EASY_PERSONAL_HOME` and `XDG_CONFIG_HOME` are temp folders, so nothing real is reached. The "secret" is a
//! canary file the test plants beside the report folder. The helpers are a trimmed copy of those of `tests/reports.rs`.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, UNIX_EPOCH};

/// Same string as `CSP` in `src/easy_personal/surfaces/api.py`.
const CSP: &str = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'";
const CANARY: &str = "CANARY-T045-HTTP-OUTSIDE";
const LIST: &str = "/api/kit/report-files?workspace=acme&repo_no=0&folder=reports";
const FILE: &str = "/api/kit/report-file?workspace=acme&repo_no=0&folder=reports";

fn parity(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity")
        .join(rel)
}

fn read_json(rel: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(parity(rel)).unwrap()).unwrap()
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

struct Server {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    port: u16,
    token: String,
    home: PathBuf,
}

impl Server {
    /// A server over a temp copy of the rich store whose workspace repos point at a temp copy of `fixtures-rich/kit`
    /// (file times fixed to the goldens'), with canaries planted outside the report folder.
    fn start() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let home = std::env::temp_dir().join(format!(
            "easy-personal-rs-t045-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        assert!(
            home.to_str().unwrap() < "/work/rich",
            "the temp folder must sort before /work/rich: set TMPDIR to a folder that does"
        );
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let kit = home.join("kit");
        copy_tree(&parity("fixtures-rich/kit"), &kit);
        std::fs::write(kit.join("secret.html"), CANARY).unwrap();
        std::fs::write(kit.join("alpha/secret.html"), CANARY).unwrap();
        let mut stack = vec![kit.join("alpha/reports"), kit.join("alpha/smoke-reports")];
        while let Some(folder) = stack.pop() {
            for entry in std::fs::read_dir(folder).unwrap() {
                let path = entry.unwrap().path();
                std::fs::File::open(&path)
                    .unwrap()
                    .set_modified(UNIX_EPOCH + Duration::from_secs(1_768_694_400))
                    .unwrap();
            }
        }
        let store = home.join("store.sqlite");
        std::fs::copy(parity("store-rich/store-v11.sqlite"), &store).unwrap();
        let conn = rusqlite::Connection::open(&store).unwrap();
        conn.execute(
            "UPDATE workspace_repos SET repo_path = replace(repo_path, '/work/rich/kit', ?1)",
            [kit.to_str().unwrap()],
        )
        .unwrap();
        drop(conn);
        let mut child = Command::new(env!("CARGO_BIN_EXE_easy-personal-rs"))
            .args(["serve", "--no-ingest"])
            .env("HOME", &home)
            .env("EASY_PERSONAL_HOME", &home)
            .env("XDG_CONFIG_HOME", home.join("xdg"))
            .env("TZ", "UTC")
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
            .unwrap_or_else(|| panic!("unexpected first line {line:?}"))
            .to_string();
        let (port, token) = url.split_once("/#").expect("URL fragment");
        Server {
            child,
            _stdout: stdout,
            port: port.parse().unwrap(),
            token: token.to_string(),
            home,
        }
    }

    fn host(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    fn raw(&self, request: &str) -> Response {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        Response::parse(&bytes)
    }

    /// `method path` with a good Host and token and the extra header lines.
    fn send(&self, method: &str, path: &str, extra: &[(&str, &str)]) -> Response {
        let mut req = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nx-dashboard-token: {}\r\nContent-Length: 0\r\n",
            self.host(),
            self.token
        );
        for (k, v) in extra {
            req += &format!("{k}: {v}\r\n");
        }
        self.raw(&(req + "Connection: close\r\n\r\n"))
    }

    fn authed(&self, path: &str) -> Response {
        self.send("GET", path, &[])
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Response {
    fn parse(bytes: &[u8]) -> Self {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((text.as_str(), ""));
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .and_then(|l| l.split(' ').nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("no status line in {text:?}"));
        let headers = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
            .collect();
        Response {
            status,
            headers,
            body: body.to_string(),
        }
    }

    fn header(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_else(|_| panic!("not JSON: {:?}", self.body))
    }

    fn assert_csp(&self) {
        assert_eq!(self.header("content-security-policy"), vec![CSP]);
    }

    /// A refusal of the route itself: the fixed body, never the input, never the canary.
    fn assert_not_found(&self, what: &str) {
        assert_eq!(self.status, 404, "{what}: {}", self.body);
        assert_eq!(self.json(), json!({"error": "not found"}), "{what}");
        assert_eq!(self.header("content-type"), vec!["application/json"]);
        assert!(!self.body.contains(CANARY), "{what}");
        self.assert_csp();
    }
}

/// `%`-encodes everything but unreserved characters (what `URLSearchParams` would send).
fn enc(text: &str) -> String {
    text.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[test]
fn the_nine_goldens_equal_the_answers_over_the_binary() {
    let s = Server::start();
    let manifest = read_json("golden-rich/manifest.json");
    let (mut lists, mut files) = (0, 0);
    for entry in manifest["entries"].as_array().unwrap() {
        let route = entry["route"].as_str().unwrap_or("");
        if route != "/api/kit/report-files" && route != "/api/kit/report-file" {
            continue;
        }
        let name = entry["file"].as_str().unwrap();
        let query: Vec<String> = entry["params"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| format!("{k}={}", enc(v.as_str().unwrap())))
            .collect();
        let r = s.authed(&format!("{route}?{}", query.join("&")));
        let golden = read_json(&format!("golden-rich/{name}"));
        r.assert_csp();
        let status = entry["status"].as_u64().map_or(200, |s| s as u16);
        assert_eq!(r.status, status, "{name}: {}", r.body);
        if route == "/api/kit/report-file" {
            files += 1;
            let Some(served) = golden.get("served_file") else {
                assert_eq!(r.json(), golden, "{name}");
                continue;
            };
            for (header, want) in served["headers"].as_object().unwrap() {
                assert_eq!(
                    r.header(header),
                    vec![want.as_str().unwrap()],
                    "{name} {header}"
                );
            }
            assert_eq!(served["text"], r.body, "{name}");
        } else {
            lists += 1;
            assert_eq!(r.json(), golden, "{name}");
            assert_eq!(r.header("content-type"), vec!["application/json"]);
        }
    }
    eprintln!(
        "http goldens: {} equal / {} total",
        lists + files,
        lists + files
    );
    assert_eq!((lists, files), (5, 4));
}

#[test]
fn the_guard_comes_first_and_write_methods_are_405() {
    let s = Server::start();
    for route in [LIST, &format!("{FILE}&path=index.html")] {
        let no_token = s.raw(&format!(
            "GET {route} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            s.host()
        ));
        assert_eq!(no_token.status, 403, "{route}");
        assert_eq!(no_token.json(), json!({"error": "forbidden"}));
        no_token.assert_csp();
        let bad_host = s.raw(&format!(
            "GET {route} HTTP/1.1\r\nHost: evil.invalid\r\nx-dashboard-token: {}\r\nConnection: close\r\n\r\n",
            s.token
        ));
        assert_eq!(bad_host.status, 403, "{route}");
        let bad_origin = s.send("GET", route, &[("Origin", "http://evil.invalid")]);
        assert_eq!(bad_origin.status, 403, "{route}");
        for method in ["POST", "PUT", "DELETE", "PATCH"] {
            let r = s.send(method, route, &[]);
            assert_eq!(r.status, 405, "{method} {route}");
            assert!(!r.body.contains("<p>"), "{method} {route}");
            r.assert_csp();
        }
    }
}

#[test]
fn head_is_get_without_a_body() {
    let s = Server::start();
    let get = s.authed(&format!("{FILE}&path=index.html"));
    assert_eq!(get.status, 200);
    let head = s.send("HEAD", &format!("{FILE}&path=index.html"), &[]);
    assert_eq!(head.status, 200);
    assert_eq!(head.body, "");
    for header in [
        "content-type",
        "content-disposition",
        "cache-control",
        "x-content-type-options",
    ] {
        assert_eq!(head.header(header), get.header(header), "{header}");
    }
    head.assert_csp();
}

#[test]
fn repeated_missing_and_odd_query_parameters_give_pythons_answers() {
    let s = Server::start();
    // the last value of a repeated key wins (Starlette's `QueryParams.get`)
    let r = s.authed("/api/kit/report-file?workspace=acme&repo_no=9&repo_no=0&folder=nope&folder=reports&path=x&path=index.html");
    assert_eq!(
        (r.status, r.body.as_str()),
        (
            200,
            "<!doctype html><title>invented</title><p>INVENTED_REPORT</p>\n"
        )
    );
    // the first of two is not the one used
    s.authed(&format!("{FILE}&path=index.html&path=nope.html"))
        .assert_not_found("last path wins");
    for query in [
        "",
        "?workspace=acme&repo_no=0&folder=reports", // no path
        "?workspace=acme&repo_no=0&path=index.html", // no folder
        "?workspace=acme&folder=reports&path=index.html", // no repo_no
        "?repo_no=0&folder=reports&path=index.html", // no workspace
        "?workspace=acme&repo_no=%D9%A1&folder=reports&path=index.html", // Arabic-Indic digit one
        "?workspace=acme&repo_no=-1&folder=reports&path=index.html",
        "?workspace=acme&repo_no=1e3&folder=reports&path=index.html",
        "?workspace=acme&repo_no=99999999999&folder=reports&path=index.html",
        "?workspace=orphan&repo_no=0&folder=reports&path=index.html",
        "?workspace=acme&repo_no=0&folder=reports&path=",
        "?workspace=acme&repo_no=0&folder=reports&path=index.svg",
        "?workspace=acme&repo_no=0&folder=reports&path=%00index.html",
        "?workspace=acme&repo_no=0&folder=reports&path=%FF.html", // not UTF-8: U+FFFD, no such file
    ] {
        s.authed(&format!("/api/kit/report-file{query}"))
            .assert_not_found(query);
    }
    s.authed("/api/kit/report-files")
        .assert_not_found("report-files with no query");
}

#[test]
fn encoded_and_double_encoded_traversal_is_a_404_and_the_canary_stays_out() {
    let s = Server::start();
    for path in [
        "..%2fsecret.html",
        "..%2f..%2fsecret.html",
        "%2e%2e%2fsecret.html",
        "%2e%2e/secret.html",
        "..%5csecret.html",
        "%252e%252e%252fsecret.html", // decoded once: the text `%2e%2e%2fsecret.html`, no such file
        "..%252fsecret.html",
        "%2fetc%2fpasswd",
        "%2Fsecret.html",
        "index.html%00.png",
        ".%2fsecret.html",
        "..%2f..%2falpha%2fsecret.html",
    ] {
        s.authed(&format!("{FILE}&path={path}"))
            .assert_not_found(path);
    }
    // the folder name is a miss too when it names a folder outside the rule, even one that exists
    for folder in [
        "..",
        ".",
        "%2e%2e%2fsmoke-reports",
        "alpha",
        "%2fetc",
        "reports%2f..%2f..%2f",
    ] {
        s.authed(&format!(
            "/api/kit/report-file?workspace=acme&repo_no=0&folder={folder}&path=secret.html"
        ))
        .assert_not_found(folder);
        s.authed(&format!(
            "/api/kit/report-files?workspace=acme&repo_no=0&folder={folder}"
        ))
        .assert_not_found(folder);
    }
}

#[test]
fn an_oversize_file_is_the_fixed_413_and_listed_as_not_openable() {
    let s = Server::start();
    std::fs::File::create(s.home.join("kit/alpha/reports/big.txt"))
        .unwrap()
        .set_len(100_000_000)
        .unwrap();
    let r = s.authed(&format!("{FILE}&path=big.txt"));
    assert_eq!(r.status, 413, "{}", r.body);
    assert_eq!(r.json(), json!({"error": "too large"}));
    assert_eq!(r.header("content-type"), vec!["application/json"]);
    r.assert_csp();
    let list = s.authed(LIST).json();
    let big = list["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == "big.txt")
        .unwrap();
    assert_eq!(
        (&big["size"], &big["openable"]),
        (&json!(100_000_000), &json!(false))
    );
}

#[test]
fn a_store_that_is_not_there_is_the_stores_503_not_a_miss() {
    let s = Server::start();
    std::fs::remove_file(s.home.join("store.sqlite")).unwrap();
    for route in [LIST.to_string(), format!("{FILE}&path=index.html")] {
        let r = s.authed(&route);
        assert_eq!(r.status, 503, "{route}: {}", r.body);
        assert!(
            !r.body.contains(s.home.to_str().unwrap()),
            "no path in the body"
        );
    }
}
