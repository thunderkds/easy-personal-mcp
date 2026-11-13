# T050: crate options for the keychain and the launcher

> **Correction (2026-11-13):** A's first measurement used `sync-secret-service`, which links `libdbus-1`; the chosen feature set is `async-secret-service` + `tokio` (pure-Rust `zbus`, only libc linked).

**Date**: 2026-11-13
**For**: the owner's gate on T050 (crate choice and licences). No crate is added by this file.
**Status**: options only. Nothing in `rust/Cargo.toml` or `rust/Cargo.lock` changed.

## What Python does today (the thing Rust must match)

- **Keychain item**: one item per connector. Service `easy-personal/<name>`, user `connector`, secret = JSON `{"command": [...], "env": {...}, "headers": {...}}`. Python's `keyring` package writes it (Secret Service on Linux). Source: `src/easy_personal/connectors/keychain.py`, `launch.py`.
- **Refusal rule**: no usable keychain means no write. Secrets never go to a file (FR-017).
- **Launcher**: Claude Code runs `easy-personal-launch exec <name>`. The launcher reads the item and starts the provider server with the secrets in its environment, never in argv (NFR-006). This is a Python console script today, so every connector already registered with Claude points to a Python program.
- **`claude` calls**: `claude mcp add/remove/get` through a subprocess (`connectors/writer.py`, `sync.py`). This needs no crate: `std::process::Command` is enough.

So T050 needs three things: a keychain client, a launcher program named `easy-personal-launch`, and the `claude` subprocess code. Only the keychain client is a crate question.

## Options for the keychain client

| | A. `keyring` crate, pure-Rust Secret Service | B. Call `secret-tool` | C. Keep a small Python launcher |
|---|---|---|---|
| New crates | **75 packages** new to the lock (133 in the probe tree; the rest are already in the lock) | **0** | 0 |
| Licences | all permissive: MIT or Apache-2.0 (most), BSD-3-Clause (1), BSD-2-Clause OR Apache-2.0 OR MIT (2). No GPL or LGPL | none | none |
| Main parts | `keyring` 3.6.3, `secret-service`, `zbus` 4.4, `zvariant`, `async-io` and friends, `rand`, `aes`, `hkdf`, `zeroize` | external program `secret-tool` (apt package `libsecret-tools`) | Python and `keyring` stay installed |
| Same items as Python | yes, if the attributes match (`service`, `username`); a test must prove it | yes: `secret-tool lookup service easy-personal/<name> username connector` | yes (it is Python) |
| Secret handling | in process memory only | secret goes in on stdin, never argv; one more process | as today |
| Works without extra install | yes | no: the owner must install `libsecret-tools`; not on macOS or Windows | needs Python |
| Cross-platform later | the same crate has macOS and Windows backends (more packages) | no | no |
| Fits the plan "Rust replaces Python" | yes | yes, with a weak link | **no**: Python stays |

Not measured: `keyring` 4.x (4.2.0 exists; its tree is not checked). The probe used `keyring` 3.6.3 with `sync-secret-service` and `crypto-rust` in a scratch project outside the repo.

## Recommendation

**Option A.** It is the only one that removes Python, needs no extra install and keeps secrets out of any child process. The cost is 75 new packages, all permissive licences, mostly one async D-Bus stack. Gate it so the review is easy:

1. Add `keyring` only to the app crate, with `default-features = false` and the two features above. Pin the version with `=`, as the project does for `futures-core`.
2. Put the keychain code behind one module (`keychain.rs`), so a later swap to Option B costs one file.
3. T050 starts with a stop-and-report step: `cargo tree` and the new package list before and after, then the Supervisor accepts or stops.
4. A cross-check test: write with Rust and read with Python's `keyring`, and the reverse, against a real Secret Service. This needs a session bus and an unlocked keyring, so it is an `#[ignore]` test the owner runs once by hand. CI cannot prove it.

If you want zero new crates, pick **B** and accept the `libsecret-tools` install. Pick C only to delay the removal.

## Other facts T050 must settle (no crate needed)

- **Launcher name**: ship a second binary `easy-personal-launch` (or a symlink to `easy-personal-rs launch`), so entries already registered with Claude keep working.
- **Existing connectors**: Rust must read what Python wrote. No migration of the keychain is needed if the attributes match.
- **Where `claude` runs**: only the subprocess calls named above, with the same arguments, an empty or minimal environment plus what Python passes, and a timeout. The tests use a fake `claude` script on a temp `PATH`.
- **Test data**: temp folders, a fake `claude`, and a fake keychain behind a trait. No real keychain item is touched in CI.
