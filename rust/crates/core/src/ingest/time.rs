//! `_utc` of `claude_code.py`: `datetime.fromisoformat(value)`, `None` when it fails or has no zone, else
//! `astimezone(UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")` (always 6 fractional digits).
//!
//! The oracle runs CPython 3.14, whose `fromisoformat` is the C one (`_datetimemodule.c`), not `_pydatetime.py`.
//! This is a port of the C path: `_sanitize_isoformat_str`, `_find_isoformat_datetime_separator`,
//! `parse_isoformat_date`, `parse_isoformat_time`, `parse_hh_mm_ss_ff`, the `24:00` roll-over and the field checks of
//! `new_datetime`. The C code walks a NUL-terminated UTF-8 buffer; [`at`] gives 0 past the end, so every look-ahead
//! reads what C reads. Proved against `corpus-time.json` (`gen_ingest_corpus.py`): offsets, 0-9 fractional digits
//! (more than 6 are cut, never rounded), offset fields that are not range-checked (`+05:99`; only |offset| < 24 h),
//! an offset of 0 whole seconds that drops its microseconds, `,` as the decimal sign, the basic form, any one-character separator (a
//! multi-byte or surrogate one too), hour-only and minute-only times, ISO week dates, `24:00`, and what fails (`z`, `:60`, a date
//! alone is naive, an empty string).
//!
//! Rust differs on one input: an aware time whose UTC value falls outside years 1-9999 (`0001-01-01T00:00:00+01:00`)
//! makes Python's `astimezone` raise `OverflowError`, which `_utc` does not catch, so the Python ingest crashes on the
//! line. Here it is `None` like any other bad timestamp (the corpus marks these `crash:OverflowError`).

use super::json::code_points;
use crate::metrics::format_stored;

const DAY_MICROS: i64 = 86_400_000_000;
/// 0001-01-01T00:00:00Z and 10000-01-01T00:00:00Z in microseconds since the Unix epoch.
const MIN_MICROS: i64 = -62_135_596_800_000_000;
const END_MICROS: i64 = 253_402_300_800_000_000;
const CORRECTION: [i32; 5] = [100_000, 10_000, 1_000, 100, 10];

/// `_utc(value)` for a string value (WTF-8 bytes of the Python `str`).
pub fn utc_text(value: &[u8]) -> Option<String> {
    let text = sanitize(value)?;
    let d = text.as_slice();
    let sep = find_separator(d)?;
    let (mut year, mut month, mut day) = parse_date(d, sep)?;
    let (mut hour, minute, second, micro, offset) = if d.len() > sep {
        // The separator is any one character: skip all of its UTF-8 bytes.
        let mut start = sep + 1;
        while at(d, start) & 0xC0 == 0x80 {
            start += 1;
        }
        parse_time(d, start)?
    } else {
        (0, 0, 0, 0, None)
    };
    // `fromisoformat`: 24:00 is midnight of the next day, but only for a date that is valid so far.
    if hour == 24 && month <= 12 {
        let dim = days_in_month(year, month);
        if day <= dim {
            if minute != 0 || second != 0 || micro != 0 {
                return None;
            }
            hour = 0;
            day += 1;
            if day > dim {
                day = 1;
                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
            }
        }
    }
    // `new_datetime` field checks.
    if !(1..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
        || !(0..=999_999).contains(&micro)
    {
        return None;
    }
    // A naive time: `_utc` returns None.
    let offset = offset?;
    let days = days_from_civil(i64::from(year), i64::from(month), i64::from(day));
    let clock = days * DAY_MICROS
        + (i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second)) * 1_000_000
        + i64::from(micro);
    let utc = clock - offset;
    // Python: `OverflowError` in `astimezone` (the ingest crashes); here an unreadable timestamp.
    (MIN_MICROS..END_MICROS)
        .contains(&utc)
        .then(|| format_stored(utc))
}

/// The byte at `i`, or 0 past the end: the C code reads a NUL-terminated buffer.
fn at(d: &[u8], i: usize) -> u8 {
    d.get(i).copied().unwrap_or(0)
}

/// `_sanitize_isoformat_str` + `PyUnicode_AsUTF8AndSize`: fewer than 7 characters fail; a surrogate at character 7,
/// 8 or 10 (the first found) becomes `T`; any other surrogate fails (it cannot be encoded as UTF-8).
fn sanitize(value: &[u8]) -> Option<Vec<u8>> {
    if value.is_ascii() {
        return (value.len() >= 7).then(|| value.to_vec());
    }
    let mut cps: Vec<u32> = code_points(value).collect();
    if cps.len() < 7 {
        return None;
    }
    let surrogate = |c: u32| (0xD800..0xE000).contains(&c);
    if let Some(p) = [7, 8, 10]
        .into_iter()
        .find(|&p| cps.get(p).is_some_and(|c| surrogate(*c)))
    {
        cps[p] = u32::from(b'T');
    }
    let mut out = String::with_capacity(value.len());
    for cp in cps {
        out.push(char::from_u32(cp)?);
    }
    Some(out.into_bytes())
}

/// `_find_isoformat_datetime_separator`; `None` where C returns -1 (that string never parses).
fn find_separator(d: &[u8]) -> Option<usize> {
    let len = d.len();
    if len == 7 {
        return Some(7);
    }
    if at(d, 4) == b'-' {
        if at(d, 5) == b'W' {
            if len < 8 {
                return None;
            }
            if len > 8 && at(d, 8) == b'-' {
                if len == 9 {
                    return None;
                }
                if len > 10 && at(d, 10).is_ascii_digit() {
                    return Some(8);
                }
                return Some(10);
            }
            return Some(8);
        }
        return Some(10);
    }
    if at(d, 4) == b'W' {
        let mut idx = 7;
        while idx < len && at(d, idx).is_ascii_digit() {
            idx += 1;
        }
        if idx < 9 {
            return Some(idx);
        }
        return Some(if idx % 2 == 0 { 7 } else { 8 });
    }
    Some(8)
}

/// `parse_digits`: `n` ASCII digits at `*p` (reading past the field as C does).
fn digits(d: &[u8], p: &mut usize, n: usize) -> Option<i32> {
    let mut v = 0i32;
    for _ in 0..n {
        let c = at(d, *p).wrapping_sub(b'0');
        if c > 9 {
            return None;
        }
        v = v * 10 + i32::from(c);
        *p += 1;
    }
    Some(v)
}

/// `parse_isoformat_date(dtstr, len = separator)`.
fn parse_date(d: &[u8], len: usize) -> Option<(i32, i32, i32)> {
    let mut p = 0;
    let year = digits(d, &mut p, 4)?;
    let dash = at(d, p) == b'-';
    if dash {
        p += 1;
    }
    if at(d, p) == b'W' {
        p += 1;
        let week = digits(d, &mut p, 2)?;
        let weekday = if p < len {
            if dash {
                let c = at(d, p);
                p += 1;
                if c != b'-' {
                    return None;
                }
            }
            digits(d, &mut p, 1)?
        } else {
            1
        };
        return iso_to_ymd(year, week, weekday);
    }
    let month = digits(d, &mut p, 2)?;
    if dash {
        let c = at(d, p);
        p += 1;
        if c != b'-' {
            return None;
        }
    }
    let day = digits(d, &mut p, 2)?;
    Some((year, month, day))
}

type Clock = (i32, i32, i32, i32, Option<i64>);

/// `parse_isoformat_time` from byte `start` to the end; the offset is in microseconds (`Some(0)` for `Z`).
fn parse_time(d: &[u8], start: usize) -> Option<Clock> {
    let end = d.len();
    // The first `Z`, `+` or `-` (C checks the first byte even of an empty time).
    let mut tz = start;
    loop {
        if matches!(at(d, tz), b'Z' | b'+' | b'-') {
            break;
        }
        tz += 1;
        if tz >= end {
            break;
        }
    }
    let ([hour, minute, second, micro], more) = hh_mm_ss_ff(d, start, tz)?;
    if tz == end {
        return (!more).then_some((hour, minute, second, micro, None));
    }
    if at(d, tz) == b'Z' {
        return (at(d, tz + 1) == 0).then_some((hour, minute, second, micro, Some(0)));
    }
    let sign: i64 = if at(d, tz) == b'-' { -1 } else { 1 };
    let ([th, tm, ts, tus], more) = hh_mm_ss_ff(d, tz + 1, end)?;
    if more {
        return None;
    }
    // The fields are not range-checked (`+05:99` is 6h39m); `tzinfo_from_isoformat_results` gives UTC when the
    // whole-second part is 0 (its microseconds are then dropped), and `timezone()` needs |offset| < 24 h.
    let seconds = i64::from(th) * 3600 + i64::from(tm) * 60 + i64::from(ts);
    if seconds == 0 {
        return Some((hour, minute, second, micro, Some(0)));
    }
    let offset = sign * (seconds * 1_000_000 + i64::from(tus));
    (offset.abs() < DAY_MICROS).then_some((hour, minute, second, micro, Some(offset)))
}

/// `parse_hh_mm_ss_ff(tstr, tstr_end)`: `HH[:?MM[:?SS[{.,}f+]]]`. The flag is C's return value 1: the text does
/// not end (no NUL) where the parse stopped.
fn hh_mm_ss_ff(d: &[u8], start: usize, end: usize) -> Option<([i32; 4], bool)> {
    let mut v = [0i32; 4];
    let mut p = start;
    let mut has_sep = true;
    for i in 0..3 {
        v[i] = digits(d, &mut p, 2)?;
        let c = at(d, p);
        p += 1;
        if i == 0 {
            has_sep = c == b':';
        }
        if p >= end {
            return Some((v, c != 0));
        }
        if has_sep && c == b':' {
            if i == 2 {
                return None;
            }
            continue;
        }
        if c == b'.' || c == b',' {
            if i < 2 {
                return None;
            }
            break;
        }
        if has_sep {
            return None;
        }
        p -= 1;
    }
    // Here `p < end`, so at least one fractional digit is read: at most 6, the rest skipped (truncated).
    let to_parse = (end - p).min(6);
    v[3] = digits(d, &mut p, to_parse)?;
    if to_parse < 6 {
        v[3] *= CORRECTION[to_parse - 1];
    }
    while at(d, p).is_ascii_digit() {
        p += 1;
    }
    Some((v, at(d, p) != 0))
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// `days_in_month` (C keeps 0 at index 0, so month 0 has 0 days).
fn days_in_month(y: i32, m: i32) -> i32 {
    match m {
        2 if is_leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    }
}

/// Days since 1970-01-01 of a proleptic Gregorian date.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// `iso_to_ymd`: ISO year, week (1-52, 53 only in long years) and weekday (1-7) to a date.
fn iso_to_ymd(year: i32, week: i32, weekday: i32) -> Option<(i32, i32, i32)> {
    if !(1..=9999).contains(&year) {
        return None;
    }
    // Days since the epoch; 1970-01-01 was a Thursday, so (days + 3) mod 7 is 0 on a Monday.
    let jan1 = days_from_civil(i64::from(year), 1, 1);
    let jan1_weekday = (jan1 + 3).rem_euclid(7); // 0 = Monday
    if !(1..53).contains(&week) {
        // A year has a week 53 when it starts on a Thursday, or on a Wednesday in a leap year.
        let long = jan1_weekday == 3 || (jan1_weekday == 2 && is_leap(year));
        if week != 53 || !long {
            return None;
        }
    }
    if !(1..8).contains(&weekday) {
        return None;
    }
    let mut week1_monday = jan1 - jan1_weekday;
    if jan1_weekday > 3 {
        week1_monday += 7;
    }
    let (y, m, d) =
        crate::metrics::civil_from_days(week1_monday + i64::from((week - 1) * 7 + weekday - 1));
    Some((y as i32, m as i32, d as i32))
}
