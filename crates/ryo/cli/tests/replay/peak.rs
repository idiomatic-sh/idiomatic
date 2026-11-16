//! What a warm run over ruff peaks at in memory, in a process of its own.

#[cfg(target_os = "linux")]
use crate::{counting, ruff};
#[cfg(target_os = "linux")]
use ryo_core::inbound::use_cases::lint::types::Rule;

/// How many percent over its best the warm peak may be: the peak is resident memory, which moves with how many workers hold their
/// tables at once, and the bench of `lint` measured a peak that differed by 0.2% between medians of three and a run alone is
/// noisier (asymptote tool `peak`).
#[cfg(target_os = "linux")]
const WARM_PEAK_TOLERANCE_PERCENT: u64 = 5;

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
fn a_warm_unchanged_run_over_ruff_peaks() {
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
                "peak::a_warm_unchanged_run_over_ruff_peaks",
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
        // The peak is a measure for asymptote's ratchet (tool `peak`), which holds its best and fails a run that rises over it.
        assert!(peak > 0, "the child measured a peak: {peak} bytes");
        crate::ratchet::print(
            "ruff",
            &[crate::ratchet::Measure {
                name: "warm-peak-bytes",
                value: peak,
                tolerance_percent: WARM_PEAK_TOLERANCE_PERCENT,
            }],
        );
    }
}
