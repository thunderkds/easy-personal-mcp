//! T055: the one path resolver (`metrics::resolve_path`, and `metrics::realpath` over text) is Python 3.14's non-strict
//! `Path.resolve()` / `os.path.realpath`: a `seen` map, no cap on links, a link met while it is being resolved is a loop
//! and stays as typed. It is replayed against the answers Python gave (`rust/parity/golden-writes/paths.json` for absolute
//! inputs, `paths-relative.json` for relative ones, over the folder `gen_writes.py` builds), plus the edge inputs the
//! corpus cannot hold. Two tests read or change the process's working folder; tests of one binary run on parallel threads,
//! so they hold `Cwd` (one mutex; the folder is put back when it drops, also after a panic) for their whole run.

use easy_personal_core::metrics::{realpath, resolve_path};
use paths_common::{Temp, links_folder};
use serde_json::Value;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, mpsc};
use std::time::{Duration, Instant};

#[path = "paths_common/mod.rs"]
mod paths_common;

static CWD: Mutex<()> = Mutex::new(());

/// The lock on the working folder, and the folder to put back (T055: a test that reads it must not see another's change).
struct Cwd {
    saved: PathBuf,
    _lock: MutexGuard<'static, ()>,
}

impl Cwd {
    fn hold() -> Cwd {
        let lock = CWD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        Cwd {
            saved: std::env::current_dir().unwrap(),
            _lock: lock,
        }
    }
}

impl Drop for Cwd {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.saved);
    }
}

fn golden(file: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity/golden-writes")
        .join(file);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Both entry points, for one input: what each answers, as text.
fn answers(input: &str) -> [(&'static str, String); 2] {
    [
        ("realpath", realpath(input)),
        (
            "resolve_path",
            resolve_path(Path::new(input))
                .to_string_lossy()
                .into_owned(),
        ),
    ]
}

#[test]
fn absolute_inputs_equal_python_over_links_loops_and_chains() {
    let work = links_folder("resolver-abs");
    let corpus = golden("paths.json");
    let mut compared = 0;
    for case in corpus["cases"].as_array().unwrap() {
        // `normalise_repo` also strips, expands `~` and refuses: only its plain absolute inputs are the resolver's
        let (Some(input), Some(ok)) = (case["input"].as_str(), case["ok"].as_str()) else {
            continue;
        };
        if !(input.starts_with('/') || input.starts_with("@W@/")) || input != input.trim() {
            continue;
        }
        let input = input.replace("@W@", work.text());
        let want = ok.replace("<WORK>", work.text());
        for (name, got) in answers(&input) {
            assert_eq!(got, want, "{name}({input})");
        }
        compared += 1;
    }
    // 53 cases, minus the 7 that are not plain absolute text (relative, empty, blank, NUL, a number, one padded)
    assert_eq!(compared, 46, "the committed file or the filter moved");
}

#[test]
fn relative_inputs_start_at_the_working_folder() {
    let work = links_folder("resolver-rel");
    let corpus = golden("paths-relative.json");
    let _cwd = Cwd::hold(); // declared after `work`: the folder is put back before `work` is removed
    std::env::set_current_dir(&*work).unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 22);
    for case in cases {
        let input = case["input"].as_str().unwrap();
        let want = case["ok"].as_str().unwrap().replace("<WORK>", work.text());
        for (name, got) in answers(input) {
            assert_eq!(got, want, "{name}({input:?})");
        }
    }
}

/// `hop_i -> hop_{i+1}/hop_{i+1}`, `hop_30 -> .` expands 2^30 times without a cache of resolved links. The work runs on a
/// thread so a regression is a failed assertion after 10 s, not a test run that never ends.
#[test]
fn the_depth_30_doubling_chain_is_answered_in_under_a_second() {
    let work = links_folder("resolver-hop");
    let input = format!("{}/hop0/x", work.text());
    let want = format!("{}/x", work.text());
    let (done, answer) = mpsc::channel();
    let started = Instant::now();
    std::thread::spawn(move || {
        let _ = done.send(answers(&input));
    });
    let got = answer
        .recv_timeout(Duration::from_secs(10))
        .expect("no answer in 10 s");
    let elapsed = started.elapsed();
    for (name, text) in got {
        assert_eq!(text, want, "{name}");
    }
    assert!(elapsed < Duration::from_secs(1), "took {elapsed:?}");
}

#[test]
fn the_edges_of_a_path() {
    let work = Temp::new("resolver-edge");
    let _cwd = Cwd::hold();
    // `..` at the root stays at the root; a doubled or trailing slash and a `.` fold away
    for (input, want) in [
        ("/", "/"),
        ("/..", "/"),
        ("/../..", "/"),
        ("//", "/"),
        ("/a/../../b", "/b"),
        ("/a//b/", "/a/b"),
        ("/a/./b/.", "/a/b"),
    ] {
        for (name, got) in answers(input) {
            assert_eq!(got, want, "{name}({input})");
        }
    }
    // an empty path is the working folder
    let cwd = std::env::current_dir().unwrap();
    assert_eq!(resolve_path(Path::new("")), cwd);
    assert_eq!(resolve_path(Path::new(".")), cwd);
    // a name that is not UTF-8 is kept byte for byte by `resolve_path` (and made lossy by `realpath`)
    let bad = std::ffi::OsStr::from_bytes(b"bad\xff");
    std::fs::create_dir(work.join(bad)).unwrap();
    symlink(work.join(bad), work.join("l")).unwrap();
    let got = resolve_path(&work.join("l").join("x"));
    assert_eq!(got, work.join(bad).join("x"));
    assert_eq!(
        realpath(&format!("{}/l/x", work.text())),
        format!("{}/bad\u{fffd}/x", work.text())
    );
    // a relative target that goes up and down, and a link met a second time (resolved once, then reused: not a loop)
    std::fs::create_dir_all(work.join("a/b")).unwrap();
    symlink("../a/b", work.join("a/up")).unwrap();
    symlink("a/up", work.join("twice")).unwrap();
    let want = work.join("a/b");
    assert_eq!(resolve_path(&work.join("twice")), want);
    assert_eq!(resolve_path(&work.join("twice/../../twice")), want);
    assert_eq!(
        resolve_path(&work.join("twice/../twice")),
        work.join("a/twice")
    );
}
