//! T044: `folders::list_dirs` against Python's own answers (`rust/parity/golden-jobs/dirs.json`, made over the tree of
//! `tree.json` by `gen_dirs_jobs.py`), and what Python does not say: a name that is not UTF-8, the 500 cap, and that
//! nothing but folder names, repo flags and the three paths ever leaves. Temp trees only.

use easy_personal_core::folders::{DirsError, list_dirs};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::{MetadataExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

fn golden(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity/golden-jobs")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

pub fn temp_root(tag: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let root = std::env::temp_dir().join(format!(
        "easy-personal-rs-t044-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&root).unwrap();
    fs::canonicalize(&root).unwrap()
}

fn build_tree(root: &Path) {
    let tree = golden("tree.json");
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

fn is_root() -> bool {
    let root = temp_root("uid");
    let uid = fs::metadata(&root).unwrap().uid();
    let _ = fs::remove_dir_all(&root);
    uid == 0
}

fn answer(path: &str) -> (u16, Value) {
    match list_dirs(Some(path)) {
        Ok(body) => (200, body),
        Err(DirsError::Invalid(text)) => (400, json!({ "error": text })),
        Err(DirsError::Unreadable) => (403, json!({ "error": "Cannot read this folder" })),
    }
}

fn replace(value: &Value, from: &str) -> Value {
    serde_json::from_str(&value.to_string().replace("<ROOT>", from)).unwrap()
}

#[test]
fn every_dirs_case_equals_python() {
    let root = temp_root("dirs");
    build_tree(&root);
    let (mut equal, mut total) = (0, 0);
    for case in golden("dirs.json")["cases"].as_array().unwrap() {
        let (name, template) = (case["case"].as_str().unwrap(), &case["path"]);
        let Some(template) = template.as_str().filter(|t| !t.is_empty()) else {
            continue; // no path: the working folder, covered over the binary in `app/tests/dirs_jobs.rs`
        };
        if case["unprivileged"] == true && is_root() {
            continue;
        }
        let locked = root.join("main/locked");
        if case["unprivileged"] == true {
            fs::set_permissions(&locked, std::os::unix::fs::PermissionsExt::from_mode(0o000))
                .unwrap();
        }
        total += 1;
        let (status, mut body) = answer(&template.replace("<ROOT>", root.to_str().unwrap()));
        fs::set_permissions(&locked, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
        let mut want = replace(&case["body"], root.to_str().unwrap());
        assert_eq!(status, case["status"], "{name}");
        if want.get("home").is_some() {
            // `home` is $HOME (a process-wide variable a test cannot set): compared to it, then left out
            assert_eq!(body["home"], std::env::var("HOME").unwrap(), "{name} home");
            body.as_object_mut().unwrap().remove("home");
            want.as_object_mut().unwrap().remove("home");
        }
        if want["error"].is_null() && status != 200 {
            assert!(body["error"].is_string(), "{name}"); // CPython's own text: status and the key only
        } else {
            assert_eq!(body, want, "{name}");
        }
        equal += 1;
    }
    println!("dirs oracle equal/total: {equal}/{total}");
    assert!(total >= 12);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_listing_holds_only_names_repo_flags_and_three_paths() {
    let root = temp_root("shape");
    build_tree(&root);
    fs::write(root.join("main/secret-file.txt"), "TOPSECRET").unwrap();
    symlink("/etc/passwd-target-name", root.join("main/dangling")).unwrap();
    let body = list_dirs(Some(root.join("main").to_str().unwrap())).unwrap();
    assert_eq!(
        body.as_object().unwrap().keys().collect::<Vec<_>>(),
        ["dirs", "home", "parent", "path"]
    );
    for d in body["dirs"].as_array().unwrap() {
        assert_eq!(
            d.as_object().unwrap().keys().collect::<Vec<_>>(),
            ["name", "repo"]
        );
    }
    let text = body.to_string();
    for hidden in [
        "secret-file",
        "TOPSECRET",
        "passwd-target-name",
        "dangling",
        ".hidden",
    ] {
        assert!(!text.contains(hidden), "{hidden}");
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn errors_name_no_path_and_an_unlisted_name_is_skipped() {
    let root = temp_root("err");
    let secret = root.join("sEcReT-folder-name");
    for path in [
        secret.to_str().unwrap().to_string(),
        format!("relative-{}", secret.display()),
        format!("{}/x\0y", root.display()),
        format!("/{}", "a".repeat(4000)),
    ] {
        let error = list_dirs(Some(&path)).unwrap_err().text();
        assert!(
            ["Path must be absolute", "Not an existing folder"].contains(&error),
            "{error}"
        );
    }
    // a name that is not UTF-8 is skipped, the rest is listed
    let name =
        <std::ffi::OsString as std::os::unix::ffi::OsStringExt>::from_vec(vec![b'b', 0xff, b'd']);
    fs::create_dir_all(root.join(&name)).unwrap();
    fs::create_dir_all(root.join("ok")).unwrap();
    let body = list_dirs(Some(root.to_str().unwrap())).unwrap();
    assert_eq!(body["dirs"], json!([{"name": "ok", "repo": false}]));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn proc_and_dev_are_listed_or_refused_without_a_panic() {
    for path in ["/proc/self", "/dev", "/"] {
        let (status, _) = answer(path);
        assert!([200, 400, 403].contains(&status), "{path}");
    }
}
