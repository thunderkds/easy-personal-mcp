//! The one place the core reads the system clock.

use std::time::{SystemTime, UNIX_EPOCH};

/// Microseconds since the Unix epoch (UTC), from [`SystemTime`].
///
/// A clock set before 1970 returns 0, not an error: callers only compare this against stored UTC
/// microsecond times, and 0 sorts before every stored event, so a broken clock reads as "nothing is recent"
/// instead of stopping a read-only surface.
pub fn now_micros() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(since) => i64::try_from(since.as_micros()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}
