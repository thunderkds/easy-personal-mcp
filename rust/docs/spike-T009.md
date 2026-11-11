# Spike T009 — the local time zone source
**Date**: 2026-11-10 · **Machine**: Ubuntu 26.04.1 LTS (WSL2), `/etc/localtime` -> `/usr/share/zoneinfo/Asia/Bangkok`, tzdata 2026e · **Task guide**: `tasks/TASK_GUIDE_T009.md`

Facts come from the crates.io API (`https://crates.io/api/v1/crates/<name>`, generic User-Agent `easy-personal-rs-spike`), `cargo info`,
the OSV API, the crate sources under `~/.cargo/registry/src` and two scratch crates outside the repo (a copy of each build is not kept).
Everything fetched is data: nothing found in it was followed as an instruction.

## Verdict

**Use `jiff` `=0.2.42`**, default features, one new dependency of `easy-personal-core`.

| Item | `jiff` | `chrono` + `iana-time-zone` + `chrono-tz` |
|---|---|---|
| Version pinned | `=0.2.42` (latest stable, 2026-11-10; Unlicense OR MIT; 271 M downloads; OSV: no advisory) | `chrono =0.4.45`, `iana-time-zone =0.1.65`, `chrono-tz =0.10.4` (2025-07-11) |
| MSRV vs 1.90 | 1.70, below 1.90. Cargo resolved it with the workspace `rust-version = "1.90"` ("highest Rust 1.90 compatible versions") | 1.62 / 1.62 / 1.65, below 1.90 |
| Honours `TZ=Asia/Bangkok` here | **yes**: `TZ=Asia/Bangkok`, `TZ=America/New_York`, `TZ=:America/New_York` (leading colon), `TZ=` (UTC) and a TZif path all gave libc's offset (scratch run, compared with Python under the same `TZ`) | **no**: `iana_time_zone::get_timezone()` printed `Asia/Bangkok` with `TZ=America/New_York` set (it reads `/etc/timezone` and `/etc/localtime`, not `TZ`), so the `TZ` rule would need our own code, plus a name lookup in `chrono-tz` |
| Time zone database | **the system one**: `/usr/share/zoneinfo`, `/usr/share/lib/zoneinfo`, `/etc/zoneinfo` (`jiff-0.2.42/src/tz/db/zoneinfo/enabled.rs:25`), the same files libc reads, so the same tzdata version as Python on that machine. A copy is bundled only on Windows and wasm (`jiff-tzdb-platform 0.1.3`, target-gated in `jiff`'s `Cargo.toml`); the data version of that copy was not checked | compiled into `chrono-tz` (frozen at the crate's release, may differ from the OS and from Python) |
| Build-time cost | clean build of the scratch crate: 2.7 s dev, 3.1 s release; adds `jiff` + `jiff-core` (+ `jiff-static` proc-macro, not used at runtime) to the core crate; release scratch binary 0.9 MB | clean build of the scratch crate: 5.0 s dev, 4.2 s release; seven crates (`phf`, `siphasher`, `num-traits`, ...); the 0.5 MB size is without `chrono-tz` data linked, so it is not comparable |
| Correct disambiguation | `compatible()` is Python's rule (below) | would be hand-written |

Chosen because it is the only candidate that reads `TZ` and the OS zone by itself, reads the system database Python reads, and is one crate.

## Evidence

- **Python's rule** for `datetime.fromisoformat(v).astimezone()` on a naive value (Python 3.14.4, `TZ=America/New_York`): `2026-03-08T02:30` (skipped)
  -> `07:30Z` (the offset before the change, so it lands at 03:30 EDT); `2026-11-01T01:30` (repeated) -> `05:30Z` (the first occurrence, `fold=0`).
  `jiff`'s `Disambiguation::Compatible` is the same rule. Written in `metrics.rs` `local_to_utc`.
- **25,088 wall-clock times** (every 15 minutes, +-6 h around each offset change 2010-2032, 15 zones: America/New_York, Europe/London, Europe/Dublin,
  Australia/Lord_Howe (30 min DST), Australia/Adelaide, America/Sao_Paulo, Pacific/Apia, Pacific/Chatham, America/St_Johns, America/Havana, Asia/Kolkata,
  Asia/Kathmandu, Asia/Tehran, Africa/Casablanca, Antarctica/Troll): Python's `astimezone(UTC)` against the same `to_ambiguous_zoned(..).compatible()` call
  as in `metrics.rs`: **0 mismatches**.
- **Python oracle** `rust/parity/golden-tz/` (`gen_golden.py --tz`): `Asia/Bangkok`, `America/New_York` (23-hour and 25-hour day, a record just before and
  after each change, an offset-less `since` in the skipped hour and in the repeated hour), and `Nowhere/Invalid`. `cargo test -p easy-personal-core --test usage_tz_parity`.
- **`TZ` values** (this machine, Python vs the core's zone source): equal for `Asia/Bangkok`, `:America/New_York`, a `/usr/share/zoneinfo/...` path,
  a `../zoneinfo/...` path, `EST5EDT4,M3.2.0,M11.1.0`, `XYZ-3`, `UTC0`, an empty `TZ`, an unknown name and `garbage!!` (all UTC).
  Different: see "Differences from libc".

## How the core reads the zone

`TimeZone::system()` once per `usage_summary` / `usage_breakdown` call. The result is cached by `jiff` for **5 minutes** (`jiff-0.2.42/src/tz/system/mod.rs`,
`TTL = 5 * 60 s`) and **cannot be reset from outside** (the `tz::db().reset()` call clears the database cache, not this one). A process therefore sees a
change of `TZ` or `/etc/localtime` up to 5 minutes late (Python on glibc is believed to read `TZ` once, at its first local-time call, and to see a later change only after `time.tzset()`; that was not tested). Consequence for the tests: one process cannot switch zones, so `usage_tz_parity.rs` starts its own test binary as a child
with `TZ` set for the child only (`#[ignore]`d child test + one helper, `run_child`); no `unsafe { set_var }`, no lock, the zone tests can run in parallel.

An unknown `TZ` falls to `jiff`'s "unknown" zone, which behaves like UTC, as libc does (found with Python: `TZ=Nowhere/Invalid` and `TZ=` give `+00:00`).
The golden `Nowhere/Invalid` is that Python run, and the Rust test equals it.

## Differences from libc (not built, listed)

1. **A POSIX string without rules**, `TZ=EST5EDT`: libc applies the default US rules (Python: `-05:00` in November, `-04:00` in July). `jiff` reads it as a
   zone *name*; on a machine that has no `EST5EDT` file (this one) it falls to UTC. With rules (`EST5EDT4,M3.2.0,M11.1.0`) or a file present they agree.
2. **A mis-cased zone name**, `TZ=asia/bangkok`: Linux libc cannot find the file and reads UTC; `jiff` matches names ignoring case and answers Bangkok.
   (macOS libc reads a case-insensitive file system, so forcing UTC here would be wrong there.)
3. **Year 1 and year 9999**: Python refuses a *local* `since`/`until` on `0001-01-01` and `9999-12-31` whatever the zone (`ValueError: year must be in 1..9999`,
   its conversion probes a day either side). The core accepts them. `jiff`'s own range is wider; a wall time it cannot convert gives `date value out of range`.

## macOS and Windows (documented, not built)

- **macOS**: `TZ` is honoured the same way (`jiff` reads it on every platform). With `TZ` unset it reads `/etc/localtime`, expecting a symlink whose target contains
  `zoneinfo/<name>` (the rule in `jiff`'s `tz/system/unix.rs`, shared with Linux); a plain file is read as an unnamed TZif. Not run on macOS.
- **Windows** (P5): `jiff` with `tz-system` calls `GetDynamicTimeZoneInformation` (via `windows-link`), maps the Windows zone name to an IANA id with a built-in copy of
  Unicode CLDR `windowsZones.xml`, and reads the bundled database (`jiff-tzdb-platform`, there is no `/usr/share/zoneinfo`). `TZ` is also honoured, "partially"
  per `jiff`'s own comment. The default features already include `tzdb-bundle-platform`, so no change to `Cargo.toml` is needed for P5. Not built or run;
  the Windows zone-name map is the part to test first.
- Defaults are kept on purpose (`std`, `tz-system`, `tz-fat`, `tzdb-bundle-platform`, `tzdb-zoneinfo`, `tzdb-concatenated`, `perf-inline`); turning features off saves little and would break P5.

## What was not done

No caching of the zone by the core (it re-asks `jiff` per call), no per-call zone argument on the public API, no Windows code, no second zone crate.
