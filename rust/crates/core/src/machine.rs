//! Machine status (T072, FR-095): one *sample* of the local machine, read from the OS only (NFR-001): CPU busy % (the delta of
//! two `/proc/stat` reads), RAM, the disk of the store's file system, the 1-minute load, the battery and the count of
//! `claude` processes. A measure the OS does not give is `None` ("no data", JSON `null`), never 0.
//!
//! Every reader takes the root it reads (`/proc`, `/sys`, a folder), so a test points it at a fake tree. Only Linux is
//! read; on any other OS [`Collector::system`] reads nothing and every measure is `None`. [`Ring`] is the in-memory history
//! of the sampler (`app/src/sampler.rs`): [`CAPACITY`] samples, the oldest dropped; nothing here writes a file or the store.

use serde_json::{Value, json};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

/// One sample every this many seconds (the sampler's interval, and `interval_s` of `GET /api/machine`).
pub const INTERVAL_S: u64 = 60;
/// Samples kept per measure: 24 h at [`INTERVAL_S`].
pub const CAPACITY: usize = 1440;

/// `/proc/stat` `cpu` line, in clock ticks: the busy ones and all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuTimes {
    pub busy: u64,
    pub total: u64,
}

/// The first battery of the machine: percent (0..=100) and whether it charges (`None` when the OS does not say).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Battery {
    pub pct: u8,
    pub charging: Option<bool>,
}

/// One reading of every measure. `t` is seconds since the epoch (UTC).
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub t: i64,
    pub cpu_pct: Option<f64>,
    pub ram_used: Option<u64>,
    pub ram_total: Option<u64>,
    pub disk_used: Option<u64>,
    pub disk_total: Option<u64>,
    pub load1: Option<f64>,
    pub battery_pct: Option<u8>,
    pub charging: Option<bool>,
    pub claude_procs: Option<u64>,
}

impl Sample {
    /// The ten keys of `GET /api/machine`'s samples; `None` is `null`.
    pub fn json(&self) -> Value {
        json!({
            "t": self.t,
            "cpu_pct": self.cpu_pct,
            "ram_used": self.ram_used,
            "ram_total": self.ram_total,
            "disk_used": self.disk_used,
            "disk_total": self.disk_total,
            "load1": self.load1,
            "battery_pct": self.battery_pct,
            "charging": self.charging,
            "claude_procs": self.claude_procs,
        })
    }
}

/// A bounded history: [`Ring::push`] past the capacity drops the oldest item. Memory stays at `capacity` items.
#[derive(Debug)]
pub struct Ring<T> {
    items: VecDeque<T>,
    capacity: usize,
}

impl<T> Ring<T> {
    pub fn new(capacity: usize) -> Ring<T> {
        Ring {
            items: VecDeque::with_capacity(capacity.min(CAPACITY)),
            capacity,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.capacity == 0 {
            return;
        }
        if self.items.len() == self.capacity {
            self.items.pop_front();
        }
        self.items.push_back(item);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The newest item.
    pub fn latest(&self) -> Option<&T> {
        self.items.back()
    }
}

impl<T: Clone> Ring<T> {
    /// A copy, oldest first.
    pub fn to_vec(&self) -> Vec<T> {
        self.items.iter().cloned().collect()
    }
}

fn read(root: &Path, rel: &str) -> Option<String> {
    fs::read_to_string(root.join(rel)).ok()
}

/// The `cpu` line of `<proc_root>/stat`. Busy = all but idle and iowait; guest time is already in user time.
pub fn read_cpu_times(proc_root: &Path) -> Option<CpuTimes> {
    let text = read(proc_root, "stat")?;
    let line = text
        .lines()
        .find(|l| l.split_whitespace().next() == Some("cpu"))?;
    let ticks: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .take(8) // user nice system idle iowait irq softirq steal
        .map(|f| f.parse().ok())
        .collect::<Option<_>>()?;
    if ticks.len() < 5 {
        return None;
    }
    let total = ticks.iter().try_fold(0u64, |sum, t| sum.checked_add(*t))?;
    let idle = ticks[3].checked_add(ticks[4])?; // at most `total`, which did not overflow
    Some(CpuTimes {
        busy: total - idle,
        total,
    })
}

/// Busy percent between two reads, one decimal. `None` when no time passed or a counter went back.
pub fn cpu_pct(prev: CpuTimes, now: CpuTimes) -> Option<f64> {
    let total = now.total.checked_sub(prev.total).filter(|t| *t > 0)?;
    let busy = now.busy.checked_sub(prev.busy)?;
    let pct = busy as f64 * 100.0 / total as f64;
    Some((pct.min(100.0) * 10.0).round() / 10.0)
}

/// `(used, total)` bytes of `<proc_root>/meminfo`: used = MemTotal - MemAvailable.
pub fn read_ram(proc_root: &Path) -> Option<(u64, u64)> {
    let text = read(proc_root, "meminfo")?;
    let kb = |name: &str| -> Option<u64> {
        let line = text.lines().find_map(|l| l.strip_prefix(name))?;
        line.strip_prefix(':')?
            .split_whitespace()
            .next()?
            .parse::<u64>()
            .ok()?
            .checked_mul(1024)
    };
    let total = kb("MemTotal")?;
    let available = kb("MemAvailable")?;
    Some((total.saturating_sub(available), total))
}

/// The 1-minute load of `<proc_root>/loadavg`.
pub fn read_load1(proc_root: &Path) -> Option<f64> {
    read(proc_root, "loadavg")?
        .split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

/// `(used, total)` bytes of the file system that holds `path`, like `df`. `None` for a missing folder and off Linux.
pub fn read_disk(path: &Path) -> Option<(u64, u64)> {
    #[cfg(target_os = "linux")]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
        // SAFETY: `c_path` is a valid NUL-terminated string; `stat` is a plain C struct that `statvfs` fills on success.
        let stat = unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(c_path.as_ptr(), &mut stat) != 0 {
                return None;
            }
            stat
        };
        // the `statvfs` field types are `u64` on x86_64 Linux and narrower on other targets
        #[allow(clippy::useless_conversion)]
        let unit = u64::from(stat.f_frsize);
        #[allow(clippy::useless_conversion)]
        let total = u64::from(stat.f_blocks).checked_mul(unit)?;
        #[allow(clippy::useless_conversion)]
        let free = u64::from(stat.f_bfree).checked_mul(unit)?;
        (total > 0).then(|| (total.saturating_sub(free), total))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        None
    }
}

/// The first machine battery under `<sys_root>/class/power_supply` (by name) that has a readable capacity.
/// A mains adapter, a peripheral (`scope` = `Device`) and a battery with no readable capacity are not it.
pub fn read_battery(sys_root: &Path) -> Option<Battery> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(sys_root.join("class/power_supply"))
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    dirs.sort();
    dirs.iter().find_map(|dir| {
        let text = |name: &str| fs::read_to_string(dir.join(name)).ok();
        if text("type")?.trim() != "Battery" || text("scope").is_some_and(|s| s.trim() == "Device")
        {
            return None;
        }
        let pct = text("capacity")?
            .trim()
            .parse::<u8>()
            .ok()
            .filter(|p| *p <= 100)?;
        let charging = match text("status").as_deref().map(str::trim) {
            Some("Charging" | "Full") => Some(true),
            Some("Discharging") => Some(false),
            _ => None,
        };
        Some(Battery { pct, charging })
    })
}

/// How many processes have the name `claude` (`<proc_root>/<pid>/comm`). A process that ends during the scan is skipped.
/// `None` when `proc_root` cannot be listed.
pub fn count_claude(proc_root: &Path) -> Option<u64> {
    let entries = fs::read_dir(proc_root).ok()?;
    let mut count = 0;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let is_pid = name
            .to_str()
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
        if !is_pid {
            continue;
        }
        if let Ok(comm) = fs::read_to_string(entry.path().join("comm"))
            && comm.trim_end_matches('\n') == "claude"
        {
            count += 1;
        }
    }
    Some(count)
}

/// Makes samples. It keeps the last CPU read, because CPU % is the delta of two reads.
pub struct Collector {
    proc_root: PathBuf,
    sys_root: PathBuf,
    disk_path: PathBuf,
    prev_cpu: Option<CpuTimes>,
}

impl Collector {
    pub fn new(proc_root: PathBuf, sys_root: PathBuf, disk_path: PathBuf) -> Collector {
        Collector {
            proc_root,
            sys_root,
            disk_path,
            prev_cpu: None,
        }
    }

    /// The real `/proc` and `/sys`, and the file system of `disk_path` (the store's folder). Off Linux those roots hold
    /// nothing, so every measure is `None`.
    pub fn system(disk_path: &Path) -> Collector {
        Collector::new("/proc".into(), "/sys".into(), disk_path.to_path_buf())
    }

    /// One reading at `t` (seconds since the epoch). The first one has no CPU %: there is no earlier read.
    pub fn sample(&mut self, t: i64) -> Sample {
        let cpu = read_cpu_times(&self.proc_root);
        let cpu_pct = self
            .prev_cpu
            .zip(cpu)
            .and_then(|(prev, now)| cpu_pct(prev, now));
        self.prev_cpu = cpu;
        let ram = read_ram(&self.proc_root);
        let disk = read_disk(&self.disk_path);
        let battery = read_battery(&self.sys_root);
        Sample {
            t,
            cpu_pct,
            ram_used: ram.map(|r| r.0),
            ram_total: ram.map(|r| r.1),
            disk_used: disk.map(|d| d.0),
            disk_total: disk.map(|d| d.1),
            load1: read_load1(&self.proc_root),
            battery_pct: battery.map(|b| b.pct),
            charging: battery.and_then(|b| b.charging),
            claude_procs: count_claude(&self.proc_root),
        }
    }
}
