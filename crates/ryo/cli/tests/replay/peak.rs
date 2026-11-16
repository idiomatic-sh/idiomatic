//! What a warm run over ruff reads, in a process of its own: nothing, when no file changed. The memory it peaks at is
//! measured by the `lint` bench (`warm-peak-megabytes`), which asymptote holds to its best.

#[cfg(target_os = "linux")]
use crate::{counting, ruff};
#[cfg(target_os = "linux")]
use ryo_core::inbound::use_cases::lint::types::Rule;

/// The argument that names the directory a child process runs the warm run over, and so marks the process as the child.
#[cfg(target_os = "linux")]
const WARM_DIRECTORY: &str = "warm-peak-directory=";

#[cfg(target_os = "linux")]
#[test]
fn a_warm_unchanged_run_over_ruff_reads_nothing_in_a_process_of_its_own() {
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
    } else {
        let copy = ruff::ruff_copy();
        let _cold = counting::lint_counting(copy.path(), rules, crate::base::store_at(copy.path()))
            .expect("the rules run");
        let child = std::process::Command::new(std::env::current_exe().expect("this test program"))
            .args([
                "--exact",
                "peak::a_warm_unchanged_run_over_ruff_reads_nothing_in_a_process_of_its_own",
                "--nocapture",
                "--test-threads=1",
            ])
            .arg(format!("{WARM_DIRECTORY}{}", copy.path().display()))
            .output()
            .expect("the child runs");
        let said = String::from_utf8_lossy(&child.stderr);
        assert!(child.status.success(), "the child failed: {said}");
    }
}
