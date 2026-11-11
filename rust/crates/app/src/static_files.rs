//! The optional static fallback of `serve --static-dir <DIR>`: the built dashboard, from the same origin as the API.
//! It sits behind the guard layer of `server.rs` (Host and Origin checked, the CSP on every answer, no token for a
//! non-`/api/` path) and follows Starlette's `StaticFiles(directory, html=True)` for `/`, `/<dir>/` and files.
//!
//! This is a new file-serving surface, so it refuses more than Starlette does:
//! - the request path is judged on its raw text, then decoded once, per segment. A backslash, a NUL, an encoded slash,
//!   backslash or NUL (`%2f`, `%5c`, `%00`), a `%25` (a second decoding), an empty or `.`-leading segment, an `api` first
//!   segment (so `/%61pi/...` never becomes a file read of `api/...`), and more than 32 segments are a 404;
//! - the joined path is canonicalised (every symlink resolved) and must stay under the canonical root: a symlink that
//!   leaves the folder is a 404, one that stays inside may be served. The check is on the resolved path, not on the text;
//! - **a name that starts with `.` is a 404** (`.env`, `.web-stamp`, `.well-known/...`), also when reached by a symlink.
//!   Differs from Python, whose `StaticFiles` serves dotfiles; the dashboard needs none;
//! - only a regular file is read (a FIFO, a socket or a device is a 404 and is never opened), files are read per
//!   request (no cache, no conditional request, no `Range`), and a file over 32 MiB is a 404: the dashboard's biggest
//!   file is far smaller, so the cap only stops a huge file from being held in memory.
//!
//! Not built: directory listings, redirects for a directory without a trailing slash, compression.

use axum::body::Body;
use axum::extract::Request;
use axum::http::header::{ALLOW, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderName, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_SEGMENTS: usize = 32;

/// The canonical form of `--static-dir`, or a one-line reason that names no path.
pub(crate) fn canonical_root(dir: &Path) -> Result<PathBuf, String> {
    match dir.canonicalize() {
        Ok(root) if root.is_dir() => Ok(root),
        _ => Err("--static-dir is not a readable directory".into()),
    }
}

/// The fallback handler: `root` is the canonical folder.
pub(crate) async fn serve(root: Arc<PathBuf>, req: Request) -> Response {
    let path = req.uri().path();
    // An `/api` path that no route serves keeps today's answer (an empty 404): never a file of that name.
    if path == "/api" || path.starts_with("/api/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let head = match *req.method() {
        Method::GET => false,
        Method::HEAD => true,
        _ => {
            return (StatusCode::METHOD_NOT_ALLOWED, [(ALLOW, "GET, HEAD")]).into_response();
        }
    };
    let Some(target) = Target::parse(path) else {
        return not_found();
    };
    // Blocking file system calls stay off the async workers.
    match tokio::task::spawn_blocking(move || read_file(&root, &target)).await {
        Ok(Some(file)) => ok(file, head),
        _ => not_found(),
    }
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        [(CONTENT_TYPE, "text/plain; charset=utf-8")],
        "Not Found",
    )
        .into_response()
}

fn ok((body, content_type): (Vec<u8>, &'static str), head: bool) -> Response {
    const NOSNIFF: (HeaderName, &str) =
        (HeaderName::from_static("x-content-type-options"), "nosniff");
    (
        StatusCode::OK,
        [
            (CONTENT_TYPE, content_type),
            (CACHE_CONTROL, "no-cache"),
            NOSNIFF,
        ],
        [(CONTENT_LENGTH, body.len().to_string())],
        if head {
            Body::empty()
        } else {
            Body::from(body)
        },
    )
        .into_response()
}

/// A request path read as folder segments; `dir` is a trailing slash (`/` and `/docs/`).
struct Target {
    segments: Vec<String>,
    dir: bool,
}

impl Target {
    fn parse(path: &str) -> Option<Target> {
        let rest = path.strip_prefix('/')?;
        let lower = rest.to_ascii_lowercase();
        if rest.contains(['\\', '\0'])
            || ["%2f", "%5c", "%00", "%25"]
                .iter()
                .any(|e| lower.contains(e))
        {
            return None;
        }
        let mut pieces: Vec<&str> = rest.split('/').collect();
        let dir = pieces.last() == Some(&"");
        if dir {
            pieces.pop();
        }
        if pieces.len() > MAX_SEGMENTS {
            return None;
        }
        let mut segments = Vec::with_capacity(pieces.len());
        for piece in pieces {
            let segment = percent_decode(piece)?;
            if segment.is_empty() || segment.starts_with('.') || segment.contains(['/', '\\', '\0'])
            {
                return None;
            }
            segments.push(segment);
        }
        if segments
            .first()
            .is_some_and(|s| s.eq_ignore_ascii_case("api"))
        {
            return None;
        }
        Some(Target { segments, dir })
    }
}

/// One decoding of `%XX` (a malformed escape or non-UTF-8 bytes is a refusal); `+` stays `+`.
fn percent_decode(piece: &str) -> Option<String> {
    let bytes = piece.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            let value = u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
            out.push(value);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The file's bytes and type, or `None` for every refusal (the caller answers one fixed 404).
fn read_file(root: &Path, target: &Target) -> Option<(Vec<u8>, &'static str)> {
    let mut path = root.to_path_buf();
    path.extend(&target.segments);
    // Canonicalising resolves every symlink; what is left must be under the canonical root, component by component
    // (`site-evil` is not under `site`).
    let mut resolved = path.canonicalize().ok()?;
    if !resolved.starts_with(root) {
        return None;
    }
    if target.dir {
        if !resolved.metadata().ok()?.is_dir() {
            return None;
        }
        resolved = resolved.join("index.html").canonicalize().ok()?;
        if !resolved.starts_with(root) {
            return None;
        }
    }
    // A symlink may point at a dotfile inside the folder: the dotfile rule applies to the resolved name too.
    let relative = resolved.strip_prefix(root).ok()?;
    if relative.components().any(|c| {
        c.as_os_str()
            .to_str()
            .is_none_or(|name| name.starts_with('.'))
    }) {
        return None;
    }
    // Only a regular file is opened: a FIFO would block the open, a directory has nothing to read.
    if !resolved.metadata().ok()?.is_file() {
        return None;
    }
    let file = std::fs::File::open(&resolved).ok()?;
    let len = file.metadata().ok()?.len();
    if len > MAX_FILE_BYTES {
        return None;
    }
    let mut body = Vec::with_capacity(len as usize);
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut body).ok()?;
    if body.len() as u64 > MAX_FILE_BYTES {
        return None;
    }
    Some((body, content_type(&resolved)))
}

/// By extension of the resolved file; anything else is `application/octet-stream`.
fn content_type(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json" | "map") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/vnd.microsoft.icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(path: &str) -> Option<(Vec<String>, bool)> {
        Target::parse(path).map(|t| (t.segments, t.dir))
    }

    #[test]
    fn a_path_is_split_decoded_once_and_checked_per_segment() {
        assert_eq!(parts("/"), Some((vec![], true)));
        assert_eq!(parts("/app.js"), Some((vec!["app.js".into()], false)));
        assert_eq!(parts("/docs/"), Some((vec!["docs".into()], true)));
        assert_eq!(
            parts("/%C3%BCn/a%2ejs"),
            Some((vec!["\u{fc}n".into(), "a.js".into()], false))
        );
        for refused in [
            "",
            "x",
            "//",
            "//a",
            "/a//b",
            "/.",
            "/..",
            "/a/../b",
            "/%2e%2e",
            "/%2E./x",
            "/.env",
            "/a/.hidden",
            "/a\\b",
            "/a%5cb",
            "/a%5Cb",
            "/a%2fb",
            "/a%2Fb",
            "/a%00",
            "/%252e%252e",
            "/%",
            "/%zz",
            "/%f",
            "/%ff",
            "/api",
            "/%61pi/x",
            "/API/x",
            "/a\0",
        ] {
            assert!(parts(refused).is_none(), "{refused:?} must be refused");
        }
        assert!(parts(&format!("/{}", "a/".repeat(MAX_SEGMENTS + 1))).is_none());
        assert!(parts(&format!("/{}", "a/".repeat(MAX_SEGMENTS))).is_some());
    }

    #[test]
    fn content_types_go_by_extension_and_default_to_octet_stream() {
        for (name, want) in [
            ("a.HTML", "text/html; charset=utf-8"),
            ("a.js", "text/javascript; charset=utf-8"),
            ("a.css", "text/css; charset=utf-8"),
            ("a.json", "application/json"),
            ("a.svg", "image/svg+xml"),
            ("a.png", "image/png"),
            ("a.ico", "image/vnd.microsoft.icon"),
            ("a.woff2", "font/woff2"),
            ("a.txt", "text/plain; charset=utf-8"),
            ("a.bin", "application/octet-stream"),
            ("noext", "application/octet-stream"),
        ] {
            assert_eq!(content_type(Path::new(name)), want, "{name}");
        }
    }
}
