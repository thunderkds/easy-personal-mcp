//! A temp folder that is removed when it drops (also when an assertion fails), and the folder of links that
//! `parity/gen_writes.py` (`path_corpus`) builds for Python, rebuilt here. Included by `writes_parity.rs` and
//! `resolver_parity.rs`.
#![allow(dead_code)] // each test uses a part

use std::ops::Deref;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

pub struct Temp(PathBuf);

impl Temp {
    /// `<temp>/easy-personal-rs-t055-<tag>-<pid>`, made new and canonical (the links below are absolute).
    pub fn new(tag: &str) -> Temp {
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t055-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Temp(std::fs::canonicalize(dir).unwrap())
    }

    pub fn text(&self) -> &str {
        self.0.to_str().unwrap()
    }
}

impl Deref for Temp {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Real, relative, dangling, looping, chained, doubling (30 hops), 3-link and self links, and (T055) chains of 41 and 60
/// links; the same folder as `path_corpus`.
pub fn links_folder(tag: &str) -> Temp {
    let work = Temp::new(tag);
    std::fs::create_dir_all(work.join("real")).unwrap();
    std::fs::create_dir(work.join("deep")).unwrap();
    std::fs::write(work.join("file"), "").unwrap();
    symlink(work.join("real"), work.join("link")).unwrap();
    symlink("real", work.join("rel")).unwrap();
    symlink(work.join("nowhere"), work.join("dangle")).unwrap();
    symlink(work.join("loop-b"), work.join("loop-a")).unwrap();
    symlink(work.join("loop-a"), work.join("loop-b")).unwrap();
    symlink(work.join("link"), work.join("chain1")).unwrap();
    symlink(work.join("deep").join(".."), work.join("dir-link")).unwrap();
    // T054: a chain of 30 links that doubles at every hop, a loop of three links, a link to itself
    for i in 0..30 {
        symlink(
            format!("hop{}/hop{}", i + 1, i + 1),
            work.join(format!("hop{i}")),
        )
        .unwrap();
    }
    symlink(".", work.join("hop30")).unwrap();
    symlink("tri-b", work.join("tri-a")).unwrap();
    symlink("tri-c", work.join("tri-b")).unwrap();
    symlink("tri-a", work.join("tri-c")).unwrap();
    symlink("selfie", work.join("selfie")).unwrap();
    // T055: `<name>-0 -> <name>-1 -> ... -> real`, `length` links
    for (name, length) in [("c41", 41), ("c60", 60)] {
        for i in 0..length - 1 {
            symlink(
                format!("{name}-{}", i + 1),
                work.join(format!("{name}-{i}")),
            )
            .unwrap();
        }
        symlink("real", work.join(format!("{name}-{}", length - 1))).unwrap();
    }
    work
}
