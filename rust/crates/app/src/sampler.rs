//! A sampler (T072, FR-095): one thread that calls `take` every `interval` (the first call is made by `start` itself), and keeps what it returns in a
//! bounded in-memory [`Ring`]. Nothing is written to disk or to the store. It knows nothing of what a sample is, so the
//! plan-limit sampler (T073, [`limits`]) and the tray reuse it. [`machine`] is the machine status sampler of `serve` and `app`.
//! Dropping the sampler stops it like [`Sampler::stop`].

use easy_personal_core::clock::now_micros;
use easy_personal_core::limits_watch::{self, Round};
use easy_personal_core::machine::{CAPACITY, Collector, INTERVAL_S, Ring, Sample};
use easy_personal_core::store::{self, StoreError, store_path};
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// The history a sampler fills and a route reads.
pub(crate) type History<T> = Arc<Mutex<Ring<T>>>;

pub(crate) struct Sampler<T> {
    history: History<T>,
    stop: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> Sampler<T> {
    /// Takes the first one now, then starts the thread: the next ones every `interval`, until [`Sampler::stop`].
    pub(crate) fn start(
        name: &str,
        capacity: usize,
        interval: Duration,
        mut take: impl FnMut() -> T + Send + 'static,
    ) -> Result<Sampler<T>, String> {
        let history: History<T> = Arc::new(Mutex::new(Ring::new(capacity)));
        // the first one is taken here, so the history is never empty once `start` returns
        let first = take();
        history
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(first);
        let (stop, stopped) = mpsc::channel::<()>();
        let shared = Arc::clone(&history);
        let thread = thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                // a timeout is the next tick; a message or a dropped sender is the stop
                while let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(interval) {
                    let item = take();
                    shared
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .push(item);
                }
            })
            .map_err(|_| format!("cannot start the {name}"))?;
        Ok(Sampler {
            history,
            stop: Some(stop),
            thread: Some(thread),
        })
    }

    pub(crate) fn history(&self) -> History<T> {
        Arc::clone(&self.history)
    }

    /// Ends the thread and joins it (it only sleeps or reads a few small files, so this is quick).
    pub(crate) fn stop(mut self) {
        self.end();
    }
}

impl<T> Drop for Sampler<T> {
    fn drop(&mut self) {
        self.end();
    }
}

impl<T> Sampler<T> {
    fn end(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The machine status sampler: every [`INTERVAL_S`] seconds one [`Sample`] of this machine, [`CAPACITY`] kept (24 h).
/// The disk measured is the file system of the store's folder (`EASY_PERSONAL_HOME` / `HOME`); with no such folder it is "no data".
pub(crate) fn machine() -> Result<Sampler<Sample>, String> {
    // An empty path is not a folder: `statvfs` fails and the disk measure is `None`.
    let folder = store_path()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_default();
    let mut collector = Collector::system(&folder);
    Sampler::start(
        "easy-personal-machine",
        CAPACITY,
        Duration::from_secs(INTERVAL_S),
        move || collector.sample(now_micros() / 1_000_000),
    )
}

/// The plan-limit sampler (T073, FR-096): every [`INTERVAL_S`] seconds one [`Round`], [`CAPACITY`] kept (24 h), on its own
/// thread `easy-personal-limits`. It opens the store read-only for the accounts; a store that cannot be read is a gap.
/// The round `Sampler::start` takes at once is a gap too, so the first real round comes after one interval: a read open
/// of the store while `serve` starts touches the `-shm` file of a WAL store, and
/// `store_health_route::the_route_writes_nothing_and_names_no_folder` (a `wal_no_shm` store whose files must keep their
/// bytes and mtimes) fails on that.
pub(crate) fn limits() -> Result<Sampler<Round>, String> {
    let mut first = true;
    limits_with(
        move || {
            if std::mem::take(&mut first) {
                Err(StoreError::NoStore)
            } else {
                store::store_path()
            }
        },
        now_micros,
        Duration::from_secs(INTERVAL_S),
    )
}

/// [`limits`] with the store, the clock (microseconds) and the interval given, for a test.
pub(crate) fn limits_with(
    mut store_path: impl FnMut() -> Result<PathBuf, StoreError> + Send + 'static,
    clock: impl Fn() -> i64 + Send + 'static,
    interval: Duration,
) -> Result<Sampler<Round>, String> {
    Sampler::start("easy-personal-limits", CAPACITY, interval, move || {
        let now = clock();
        match store_path().and_then(|path| store::open_checked(&path)) {
            Ok(conn) => limits_watch::round(&conn, now),
            Err(_) => Round::gap(now),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn wait_for(history: &History<u32>, at_least: usize) {
        let end = Instant::now() + Duration::from_secs(10);
        while history.lock().unwrap().len() < at_least {
            assert!(Instant::now() < end, "the sampler did not tick");
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn it_takes_one_at_once_then_one_per_interval_in_order() {
        let mut n = 0;
        let sampler = Sampler::start("test-sampler", 100, Duration::from_millis(10), move || {
            n += 1;
            n
        })
        .unwrap();
        let history = sampler.history();
        wait_for(&history, 3);
        sampler.stop();
        let items = history.lock().unwrap().to_vec();
        assert_eq!(items[..3], [1, 2, 3]);
    }

    #[test]
    fn it_keeps_only_the_capacity_and_stops_taking_after_stop() {
        let mut n = 0;
        let sampler = Sampler::start("test-sampler", 4, Duration::from_millis(2), move || {
            n += 1;
            n
        })
        .unwrap();
        let history = sampler.history();
        wait_for(&history, 4);
        thread::sleep(Duration::from_millis(30));
        sampler.stop();
        let after = history.lock().unwrap().to_vec();
        assert_eq!(after.len(), 4, "bounded");
        assert!(after[0] > 1, "the oldest were dropped: {after:?}");
        assert!(after.windows(2).all(|w| w[0] + 1 == w[1]));
        thread::sleep(Duration::from_millis(30));
        assert_eq!(
            history.lock().unwrap().to_vec(),
            after,
            "no tick after stop"
        );
    }

    #[test]
    fn a_long_interval_does_not_delay_stop() {
        let sampler =
            Sampler::start("test-sampler", 4, Duration::from_secs(3600), || 1u32).unwrap();
        let history = sampler.history();
        wait_for(&history, 1);
        let started = Instant::now();
        sampler.stop();
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn the_machine_sampler_has_a_sample_soon_and_stops() {
        let sampler = machine().unwrap();
        let history = sampler.history();
        let end = Instant::now() + Duration::from_secs(10);
        while history.lock().unwrap().is_empty() {
            assert!(Instant::now() < end);
            thread::sleep(Duration::from_millis(5));
        }
        sampler.stop();
    }
}
