//! Watch 2 (T073, FR-096): the plan-limit watch. Each tick of the app's sampler (`sampler.rs`, one thread, the machine
//! sampler's interval and capacity: 60 s, 1,440 = 24 h) takes one [`Round`]: for every Claude account the 5 h and 7 d
//! used % of `<folder>/easy-personal-limits.json`, read through [`accounts::limit_percents`] (the one reader; missing,
//! broken or stale = `null`). [`history`] turns the kept rounds into the per-account series. Nothing is written to disk,
//! and no account folder is written.

use crate::accounts::{self, LimitPercents};
use crate::machine::INTERVAL_S;
use rusqlite::Connection;
use serde_json::{Value, json};

/// What one tick saw. `rows` is `None` when the accounts could not be read (store busy, missing or newer): that round
/// is a gap and is not shown.
#[derive(Clone, Debug)]
pub struct Round {
    /// Seconds since the epoch (UTC), like `machine::Sample::t`.
    pub t: i64,
    pub rows: Option<Vec<LimitPercents>>,
}

impl Round {
    /// A round that read nothing.
    pub fn gap(now: i64) -> Round {
        Round {
            t: now / 1_000_000,
            rows: None,
        }
    }
}

/// One round from the store's accounts. `now` is microseconds since the epoch.
pub fn round(conn: &Connection, now: i64) -> Round {
    Round {
        t: now / 1_000_000,
        rows: accounts::limit_percents(conn, now).ok(),
    }
}

/// `{accounts: [{id, label, samples: [{t, five_h_pct, seven_d_pct}]}], interval_s}`, oldest sample first. The accounts
/// are those of the newest round that read; an account that is gone from it shows nothing, and one added later starts at
/// its first round.
pub fn history(rounds: &[Round]) -> Value {
    let seen = || {
        rounds
            .iter()
            .filter_map(|r| r.rows.as_ref().map(|rows| (r.t, rows)))
    };
    let accounts: Vec<Value> = seen()
        .next_back()
        .map(|(_, rows)| rows.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|account| {
            let samples: Vec<Value> = seen()
                .filter_map(|(t, rows)| {
                    let p = rows.iter().find(|p| p.id == account.id)?;
                    Some(
                        json!({ "t": t, "five_h_pct": p.five_h_pct, "seven_d_pct": p.seven_d_pct }),
                    )
                })
                .collect();
            json!({ "id": account.id, "label": account.label, "samples": samples })
        })
        .collect();
    json!({ "accounts": accounts, "interval_s": INTERVAL_S })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::{CAPACITY, Ring};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    const NOW: i64 = 1_800_000_000 * 1_000_000;

    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static N: AtomicU32 = AtomicU32::new(0);
            let dir = std::env::temp_dir().join(format!(
                "easy-personal-rs-t073-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir_all(&dir).unwrap();
            Temp(dir)
        }
        fn folder(&self, name: &str, limits: Option<&str>) -> PathBuf {
            let dir = self.0.join(name);
            fs::create_dir_all(&dir).unwrap();
            if let Some(text) = limits {
                fs::write(dir.join("easy-personal-limits.json"), text).unwrap();
            }
            dir
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn store(rows: &[(i64, &str, &str, Option<&str>, &Path)]) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE agent_accounts (id INTEGER PRIMARY KEY, provider TEXT, folder TEXT, label TEXT, \
             status TEXT, config_dir_env TEXT)",
        )
        .unwrap();
        for (id, provider, status, label, folder) in rows {
            conn.execute(
                "INSERT INTO agent_accounts VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
                rusqlite::params![id, provider, folder.to_string_lossy(), label, status],
            )
            .unwrap();
        }
        conn
    }

    fn snapshot(five: f64, five_reset: i64, seven: f64, seven_reset: i64) -> String {
        format!(
            r#"{{"five_hour":{{"used_percentage":{five},"resets_at":{five_reset}}},
                "seven_day":{{"used_percentage":{seven},"resets_at":{seven_reset}}},"written_at":1799999990}}"#
        )
    }

    /// What the app's sampler does: one round per tick into a ring of 1,440.
    struct LimitsWatch(Ring<Round>);
    impl LimitsWatch {
        fn new() -> Self {
            LimitsWatch(Ring::new(CAPACITY))
        }
        fn tick(&mut self, conn: &Connection, now: i64) {
            self.0.push(round(conn, now));
        }
        fn history(&self) -> Value {
            history(&self.0.to_vec())
        }
    }

    fn samples(watch: &LimitsWatch, id: i64) -> Vec<Value> {
        watch.history()["accounts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == id)
            .map(|a| a["samples"].as_array().unwrap().clone())
            .unwrap_or_default()
    }

    #[test]
    fn a_fresh_a_stale_and_a_missing_snapshot_give_values_null_and_null() {
        let tmp = Temp::new();
        let now_s = NOW / 1_000_000;
        let fresh = tmp.folder(
            "fresh",
            Some(&snapshot(42.5, now_s + 3600, 10.0, now_s + 86400)),
        );
        // five_hour reset passed: that window is null; seven_day still fresh
        let stale = tmp.folder(
            "stale",
            Some(&snapshot(80.0, now_s - 1, 20.0, now_s + 86400)),
        );
        let missing = tmp.folder("missing", None);
        let broken = tmp.folder("broken", Some("{not json"));
        let conn = store(&[
            (1, "claude", "confirmed", Some("Work"), &fresh),
            (2, "claude", "confirmed", None, &stale),
            (3, "claude", "confirmed", Some("Gone"), &missing),
            (4, "claude", "confirmed", Some("Broken"), &broken),
        ]);
        let mut watch = LimitsWatch::new();
        watch.tick(&conn, NOW);
        let first = &samples(&watch, 1)[0];
        assert_eq!(first["t"], NOW / 1_000_000);
        assert_eq!(first["five_h_pct"], 42.5);
        assert_eq!(first["seven_d_pct"], 10.0);
        let second = &samples(&watch, 2)[0];
        assert!(second["five_h_pct"].is_null());
        assert_eq!(second["seven_d_pct"], 20.0);
        for id in [3, 4] {
            let s = &samples(&watch, id)[0];
            assert!(
                s["five_h_pct"].is_null() && s["seven_d_pct"].is_null(),
                "{s}"
            );
        }
        let body = watch.history();
        assert_eq!(body["interval_s"], 60);
        assert_eq!(body["accounts"][0]["label"], "Work");
        assert_eq!(body["accounts"][1]["label"], "claude");
    }

    #[test]
    fn a_percent_over_100_is_kept_as_read() {
        let tmp = Temp::new();
        let f = tmp.folder(
            "a",
            Some(&snapshot(
                130.0,
                NOW / 1_000_000 + 60,
                5.0,
                NOW / 1_000_000 + 60,
            )),
        );
        let conn = store(&[(1, "claude", "confirmed", None, &f)]);
        let mut watch = LimitsWatch::new();
        watch.tick(&conn, NOW);
        assert_eq!(samples(&watch, 1)[0]["five_h_pct"], 130.0);
    }

    #[test]
    fn the_buffer_keeps_1440_samples_and_drops_the_oldest() {
        let tmp = Temp::new();
        let f = tmp.folder("a", None);
        let conn = store(&[(1, "claude", "confirmed", None, &f)]);
        let mut watch = LimitsWatch::new();
        for i in 0..(CAPACITY as i64 + 1) {
            watch.tick(&conn, NOW + i * 60_000_000);
        }
        let kept = samples(&watch, 1);
        assert_eq!(kept.len(), 1440);
        assert_eq!(kept[0]["t"], NOW / 1_000_000 + 60);
        assert_eq!(kept[1439]["t"], NOW / 1_000_000 + 1440 * 60);
    }

    #[test]
    fn an_added_account_appears_and_a_removed_or_ignored_one_loses_its_buffer() {
        let tmp = Temp::new();
        let a = tmp.folder("a", None);
        let b = tmp.folder("b", None);
        let conn = store(&[(1, "claude", "confirmed", None, &a)]);
        let mut watch = LimitsWatch::new();
        watch.tick(&conn, NOW);
        assert_eq!(watch.history()["accounts"].as_array().unwrap().len(), 1);
        conn.execute(
            "INSERT INTO agent_accounts VALUES (2, 'claude', ?1, 'B', 'confirmed', NULL)",
            [b.to_string_lossy()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_accounts VALUES (3, 'codex', ?1, 'C', 'confirmed', NULL)",
            [b.to_string_lossy()],
        )
        .unwrap();
        watch.tick(&conn, NOW + 60_000_000);
        assert_eq!(samples(&watch, 1).len(), 2);
        assert_eq!(samples(&watch, 2).len(), 1);
        assert!(
            samples(&watch, 3).is_empty(),
            "codex has no plan-limit file"
        );
        conn.execute(
            "UPDATE agent_accounts SET status = 'ignored' WHERE id = 1",
            [],
        )
        .unwrap();
        watch.tick(&conn, NOW + 120_000_000);
        assert!(samples(&watch, 1).is_empty());
        assert_eq!(samples(&watch, 2).len(), 2);
    }

    #[test]
    fn a_tick_writes_nothing_into_the_account_folder() {
        let tmp = Temp::new();
        let text = snapshot(1.0, NOW / 1_000_000 + 60, 2.0, NOW / 1_000_000 + 60);
        let f = tmp.folder("a", Some(&text));
        let conn = store(&[(1, "claude", "confirmed", None, &f)]);
        LimitsWatch::new().tick(&conn, NOW);
        let names: Vec<_> = fs::read_dir(&f)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["easy-personal-limits.json"]);
        assert_eq!(
            fs::read_to_string(f.join("easy-personal-limits.json")).unwrap(),
            text
        );
    }

    #[test]
    fn a_round_that_could_not_read_the_accounts_is_a_gap_not_a_removal() {
        let tmp = Temp::new();
        let f = tmp.folder("a", None);
        let conn = store(&[(1, "claude", "confirmed", None, &f)]);
        let mut watch = LimitsWatch::new();
        watch.tick(&conn, NOW);
        watch.0.push(Round::gap(NOW + 60_000_000));
        watch.tick(&conn, NOW + 120_000_000);
        assert_eq!(samples(&watch, 1).len(), 2, "the gap shows nothing");
        assert_eq!(samples(&watch, 1)[1]["t"], NOW / 1_000_000 + 120);
    }
}
