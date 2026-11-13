//! What a warm run over ruff peaks at in memory, in a process of its own.

#[cfg(target_os = "linux")]
use crate::{counting, ruff};
#[cfg(target_os = "linux")]
use ryo_core::inbound::use_cases::lint::types::Rule;

/// The most memory, in bytes, a warm run over ruff that finds nothing changed may peak at, in a test process of its own: a regression
/// guard at the peak measured (112-116 MB at checkpoint 336, rounded up to 125 MB; raised by the owner, who said the warm peak must not
/// block). The goal is the peak of the release build's run (87 MB), and this bound is lowered when the run reaches it.
#[cfg(target_os = "linux")]
const WARM_PEAK_BYTES_BOUND: u64 = 125_000_000;

/// The argument that names the directory a child process runs the warm run over, and so marks the process as the child.
#[cfg(target_os = "linux")]
const WARM_DIRECTORY: &str = "warm-peak-directory=";

/// What the child prints before the peak it measured, in bytes.
#[cfg(target_os = "linux")]
const PEAK_LINE: &str = "peak-bytes ";

/// The peak memory of this process so far, in bytes.
#[cfg(target_os = "linux")]
fn peak_bytes() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("the process status");
    let kilobytes = status
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmHWM:")?
                .trim()
                .strip_suffix("kB")?
                .trim()
                .parse::<u64>()
                .ok()
        })
        .expect("the status holds the peak in kilobytes");
    kilobytes << 10_u32
}

#[cfg(target_os = "linux")]
#[test]
fn a_warm_unchanged_run_over_ruff_peaks_within_its_bound() {
    let _beside = crate::beside();
    let rules: Vec<Rule> = counting::runnable()
        .into_iter()
        .map(|(_, rule)| rule)
        .collect();
    let named = std::env::args().find_map(|argument| {
        argument
            .strip_prefix(WARM_DIRECTORY)
            .map(std::path::PathBuf::from)
    });
    if let Some(directory) = named {
        // The child: the warm run alone, so the peak is its own.
        let warm = counting::lint_counting(&directory, rules, crate::base::store_at(&directory))
            .expect("the rules run");
        assert_eq!(warm.reads, 0, "a warm run reads no file");
        eprintln!("{PEAK_LINE}{}", peak_bytes());
    } else {
        let copy = ruff::ruff_copy();
        let _cold = counting::lint_counting(copy.path(), rules, crate::base::store_at(copy.path()))
            .expect("the rules run");
        let child = std::process::Command::new(std::env::current_exe().expect("this test program"))
            .args([
                "--exact",
                "peak::a_warm_unchanged_run_over_ruff_peaks_within_its_bound",
                "--nocapture",
                "--test-threads=1",
            ])
            .arg(format!("{WARM_DIRECTORY}{}", copy.path().display()))
            .output()
            .expect("the child runs");
        let said = String::from_utf8_lossy(&child.stderr);
        assert!(child.status.success(), "the child failed: {said}");
        let peak: u64 = said
            .lines()
            .find_map(|line| line.strip_prefix(PEAK_LINE)?.trim().parse().ok())
            .expect("the child printed its peak");
        assert!(
            peak <= WARM_PEAK_BYTES_BOUND,
            "a warm run over ruff peaks within {WARM_PEAK_BYTES_BOUND} bytes: {peak} bytes"
        );
    }
}
