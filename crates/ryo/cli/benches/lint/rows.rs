//! The rows a person reads, on stderr: each set of rules over ruff in a process of its own (the prototype's eight rules,
//! all of them, then each rule), reporting wall, CPU and peak memory against the prototype's. A rule whose relations do not
//! exist yet is reported as pending, never skipped silently. None of it is in the Bencher Metric Format document.

use std::process::Command;

use crate::counting;

/// What the prototype measured on ruff at 10 workers: wall median 370 ms, CPU about 0.7 s, peak memory 145 to 157 MB.
const SPIKE_ROW: &str = "wall 370 ms (310 to 580), CPU about 700 ms, peak 145 to 157 MB";

/// Today's ryo under Tastemaker, on the same pin: the rules took 41.1 s.
const TODAY_ROW: &str = "wall 41.1 s";

/// The load average of the machine now, as the kernel reports it.
fn load() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .unwrap_or_else(|error| format!("unavailable ({error})"))
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The ruff rows: each set in a process of its own, so the peak is its own.
pub(crate) fn sets() {
    eprintln!(
        "diagnostic-only rule subsets, release, cold; no headline benchmark metrics; load average: {}",
        load()
    );
    eprintln!(
        "{:<8} {:>5} {:>8} {:>9} {:>9} {:>8}  rules that ran",
        "set", "rules", "rows", "wall ms", "CPU ms", "peak MB"
    );
    let rules: Vec<String> = counting::runnable()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    let sets: Vec<&str> = ["spike", "all"]
        .into_iter()
        .chain(rules.iter().map(String::as_str))
        .collect();
    for set in sets {
        let output = Command::new(std::env::current_exe().expect("this bench"))
            .args(["--measure", set])
            .output()
            .expect("the measuring process runs");
        assert!(
            output.status.success(),
            "measurement {set} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8_lossy(&output.stdout);
        let line = text
            .lines()
            .find(|line| line.starts_with(&format!("{set}\t")))
            .expect("the requested rule set reports a measurement");
        let cells: Vec<&str> = line.split('\t').collect();
        if let [_, ran, names, rows, wall, cpu, peak] = cells.as_slice() {
            let number = |value: &str| {
                value
                    .parse::<u128>()
                    .expect("the measurement is an unsigned integer")
            };
            let (ran, rows, wall, cpu, peak) = (
                number(ran),
                number(rows),
                number(wall),
                number(cpu),
                number(peak),
            );
            eprintln!("{set:<8} {ran:>5} {rows:>8} {wall:>9} {cpu:>9} {peak:>8}  {names}");
        } else {
            panic!("malformed rule-set measurement: {line}");
        }
    }
    eprintln!("spike's eight rules: {SPIKE_ROW}");
    eprintln!("today's ryo under Tastemaker, all 52 rules: {TODAY_ROW}");
}
