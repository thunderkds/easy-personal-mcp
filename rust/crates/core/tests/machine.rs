//! T072 (FR-095): the machine status readers over a fake `/proc` and `/sys` tree in a temp folder (never the real ones, never
//! `~/.easy-personal`), the CPU delta, the ring buffer and the sample JSON.

use easy_personal_core::machine::{
    CAPACITY, Collector, CpuTimes, Ring, count_claude, cpu_pct, read_battery, read_cpu_times,
    read_disk, read_load1, read_ram,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

struct Fake {
    dir: PathBuf,
}

impl Fake {
    fn new() -> Fake {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "easy-personal-rs-t072-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(dir.join("proc")).unwrap();
        fs::create_dir_all(dir.join("sys/class/power_supply")).unwrap();
        Fake { dir }
    }
    fn proc(&self) -> PathBuf {
        self.dir.join("proc")
    }
    fn sys(&self) -> PathBuf {
        self.dir.join("sys")
    }
    fn put(&self, rel: &str, text: &str) {
        let path = self.dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn battery(&self, name: &str, kind: &str, capacity: &str, status: &str) {
        let base = format!("sys/class/power_supply/{name}");
        self.put(&format!("{base}/type"), &format!("{kind}\n"));
        if !capacity.is_empty() {
            self.put(&format!("{base}/capacity"), &format!("{capacity}\n"));
        }
        if !status.is_empty() {
            self.put(&format!("{base}/status"), &format!("{status}\n"));
        }
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

const STAT_A: &str = "cpu  100 0 50 800 50 0 0 0 0 0\ncpu0 100 0 50 800 50 0 0 0 0 0\nintr 1\n";
// 150 more busy ticks (user 100 + system 50), 50 more idle: 200 total, 75 % busy
const STAT_B: &str = "cpu  200 0 100 850 50 0 0 0 0 0\ncpu0 1 1 1 1 1 1 1 1 1 1\n";

#[test]
fn cpu_times_parse_and_percent_is_the_delta_of_two_reads() {
    let fake = Fake::new();
    fake.put("proc/stat", STAT_A);
    let a = read_cpu_times(&fake.proc()).unwrap();
    assert_eq!(
        a,
        CpuTimes {
            busy: 150,
            total: 1000
        }
    );
    fake.put("proc/stat", STAT_B);
    let b = read_cpu_times(&fake.proc()).unwrap();
    assert_eq!(cpu_pct(a, b), Some(75.0));
}

#[test]
fn cpu_percent_is_null_when_the_counters_did_not_move_or_went_back() {
    let a = CpuTimes {
        busy: 10,
        total: 100,
    };
    assert_eq!(cpu_pct(a, a), None);
    let back = CpuTimes { busy: 5, total: 90 };
    assert_eq!(cpu_pct(a, back), None);
}

#[test]
fn cpu_times_are_null_for_a_missing_or_garbled_file() {
    let fake = Fake::new();
    assert_eq!(read_cpu_times(&fake.proc()), None);
    fake.put("proc/stat", "cpu  a b c\n");
    assert_eq!(read_cpu_times(&fake.proc()), None);
    fake.put("proc/stat", "intr 1 2 3\n");
    assert_eq!(read_cpu_times(&fake.proc()), None);
}

#[test]
fn ram_is_total_minus_available_in_bytes() {
    let fake = Fake::new();
    fake.put(
        "proc/meminfo",
        "MemTotal:       16000 kB\nMemFree:         1000 kB\nMemAvailable:    4000 kB\nBuffers: 5 kB\n",
    );
    assert_eq!(read_ram(&fake.proc()), Some((12000 * 1024, 16000 * 1024)));
}

#[test]
fn ram_is_null_without_mem_available_or_with_no_file() {
    let fake = Fake::new();
    assert_eq!(read_ram(&fake.proc()), None);
    fake.put("proc/meminfo", "MemTotal: 16000 kB\nMemFree: 1 kB\n");
    assert_eq!(read_ram(&fake.proc()), None);
}

#[test]
fn load1_is_the_first_field() {
    let fake = Fake::new();
    assert_eq!(read_load1(&fake.proc()), None);
    fake.put("proc/loadavg", "0.52 0.40 0.30 1/200 12345\n");
    assert_eq!(read_load1(&fake.proc()), Some(0.52));
    fake.put("proc/loadavg", "nope\n");
    assert_eq!(read_load1(&fake.proc()), None);
}

#[test]
fn battery_is_null_with_no_battery_and_never_zero() {
    let fake = Fake::new();
    assert_eq!(read_battery(&fake.sys()), None); // empty power_supply (a container, WSL)
    fake.battery("AC", "Mains", "", "");
    assert_eq!(read_battery(&fake.sys()), None); // a mains adapter is not a battery
    fs::remove_dir_all(fake.sys().join("class")).unwrap();
    assert_eq!(read_battery(&fake.sys()), None); // no folder at all
}

#[test]
fn battery_percent_and_charging() {
    let fake = Fake::new();
    fake.battery("AC", "Mains", "", "");
    fake.battery("BAT0", "Battery", "73", "Charging");
    let b = read_battery(&fake.sys()).unwrap();
    assert_eq!((b.pct, b.charging), (73, Some(true)));
    fake.battery("BAT0", "Battery", "40", "Discharging");
    let b = read_battery(&fake.sys()).unwrap();
    assert_eq!((b.pct, b.charging), (40, Some(false)));
    fake.battery("BAT0", "Battery", "100", "Full");
    assert_eq!(read_battery(&fake.sys()).unwrap().charging, Some(true));
    fake.battery("BAT0", "Battery", "50", "Unknown");
    assert_eq!(read_battery(&fake.sys()).unwrap().charging, None);
    fake.battery("BAT0", "Battery", "50", "");
    assert_eq!(read_battery(&fake.sys()).unwrap().charging, None);
}

#[test]
fn a_battery_with_an_unreadable_capacity_is_skipped_and_a_peripheral_is_not_the_machine() {
    let fake = Fake::new();
    fake.battery("BAT0", "Battery", "", "Charging"); // no capacity file
    assert_eq!(read_battery(&fake.sys()), None);
    fake.battery("BAT0", "Battery", "abc", "Charging");
    assert_eq!(read_battery(&fake.sys()), None);
    fake.battery("BAT0", "Battery", "250", "Charging"); // out of range
    assert_eq!(read_battery(&fake.sys()), None);
    fake.battery("hid-mouse", "Battery", "15", "Discharging");
    fake.put("sys/class/power_supply/hid-mouse/scope", "Device\n");
    assert_eq!(read_battery(&fake.sys()), None);
    fake.battery("BAT1", "Battery", "61", "Discharging");
    assert_eq!(read_battery(&fake.sys()).unwrap().pct, 61);
}

#[test]
fn claude_processes_are_counted_by_comm_and_a_vanished_one_is_skipped() {
    let fake = Fake::new();
    fake.put("proc/101/comm", "claude\n");
    fake.put("proc/102/comm", "claude\n");
    fake.put("proc/103/comm", "bash\n");
    fake.put("proc/104/comm", "claude-helper\n");
    fs::create_dir_all(fake.proc().join("105")).unwrap(); // pid gone: no comm file
    fake.put("proc/self/comm", "claude\n"); // not a pid
    fake.put("proc/loadavg", "claude\n");
    assert_eq!(count_claude(&fake.proc()), Some(2));
}

#[test]
fn claude_processes_are_null_when_proc_cannot_be_read_and_zero_when_none_run() {
    let fake = Fake::new();
    assert_eq!(count_claude(&fake.dir.join("missing")), None);
    fake.put("proc/1/comm", "init\n");
    assert_eq!(count_claude(&fake.proc()), Some(0));
}

#[cfg(target_os = "linux")]
#[test]
fn disk_is_used_and_total_of_the_folders_file_system_and_null_for_a_missing_folder() {
    let fake = Fake::new();
    let (used, total) = read_disk(&fake.dir).unwrap();
    assert!(total > 0 && used <= total, "{used} of {total}");
    assert_eq!(read_disk(&fake.dir.join("not-there")), None);
}

#[test]
fn a_collector_makes_a_sample_of_every_measure_and_cpu_needs_two_reads() {
    let fake = Fake::new();
    fake.put("proc/stat", STAT_A);
    fake.put("proc/meminfo", "MemTotal: 2000 kB\nMemAvailable: 500 kB\n");
    fake.put("proc/loadavg", "1.25 0 0 1/1 1\n");
    fake.put("proc/7/comm", "claude\n");
    fake.battery("BAT0", "Battery", "88", "Discharging");
    let mut collector = Collector::new(fake.proc(), fake.sys(), fake.dir.clone());
    let first = collector.sample(1_000);
    assert_eq!(first.t, 1_000);
    assert_eq!(first.cpu_pct, None, "no earlier read: no data, not 0");
    assert_eq!(first.ram_used, Some(1500 * 1024));
    assert_eq!(first.ram_total, Some(2000 * 1024));
    assert_eq!(first.load1, Some(1.25));
    assert_eq!(first.battery_pct, Some(88));
    assert_eq!(first.charging, Some(false));
    assert_eq!(first.claude_procs, Some(1));
    fake.put("proc/stat", STAT_B);
    let second = collector.sample(1_060);
    assert_eq!(second.cpu_pct, Some(75.0));
    let json = second.json();
    assert_eq!(json["t"], 1_060);
    assert_eq!(json["cpu_pct"], 75.0);
    assert_eq!(json["battery_pct"], 88);
    assert_eq!(json["charging"], false);
}

#[test]
fn a_collector_over_an_empty_tree_is_all_null_and_the_json_keeps_every_key() {
    let fake = Fake::new();
    let mut collector = Collector::new(fake.proc(), fake.sys(), fake.dir.join("not-there"));
    let sample = collector.sample(5);
    let json = sample.json();
    let keys = [
        "cpu_pct",
        "ram_used",
        "ram_total",
        "disk_used",
        "disk_total",
        "load1",
        "battery_pct",
        "charging",
    ];
    for key in keys {
        assert!(json[key].is_null(), "{key} must be null: {json}");
    }
    // /proc is readable but holds no pid folder: that is a real zero, not "no data"
    assert_eq!(json["claude_procs"], 0);
    assert_eq!(json["t"], 5);
    assert_eq!(json.as_object().unwrap().len(), 10);
}

#[test]
fn a_cpu_read_that_fails_forgets_the_earlier_one() {
    let fake = Fake::new();
    fake.put("proc/stat", STAT_A);
    let mut collector = Collector::new(fake.proc(), fake.sys(), fake.dir.clone());
    collector.sample(1);
    fs::remove_file(fake.proc().join("stat")).unwrap();
    assert_eq!(collector.sample(2).cpu_pct, None);
    fake.put("proc/stat", STAT_B);
    assert_eq!(collector.sample(3).cpu_pct, None, "the delta spans a gap");
}

#[test]
fn the_ring_keeps_1440_samples_drops_the_oldest_and_keeps_the_order() {
    assert_eq!(CAPACITY, 1440);
    let mut ring = Ring::new(CAPACITY);
    assert!(ring.is_empty() && ring.latest().is_none());
    for n in 0..=CAPACITY {
        ring.push(n); // 1,441 pushes
    }
    assert_eq!(ring.len(), CAPACITY);
    let all = ring.to_vec();
    assert_eq!(all.first(), Some(&1), "the oldest (0) is dropped");
    assert_eq!(all.last(), Some(&CAPACITY));
    assert!(all.windows(2).all(|w| w[0] + 1 == w[1]));
    assert_eq!(ring.latest(), Some(&CAPACITY));
}

#[test]
fn the_default_collector_reads_the_real_machine_without_panicking() {
    // Only shape: the values depend on the host.
    let mut collector = Collector::system(Path::new("."));
    let json = collector.sample(1).json();
    assert_eq!(json.as_object().unwrap().len(), 10);
}
