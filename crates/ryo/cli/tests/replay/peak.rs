//! What a warm run reads in a process of its own: nothing, when no file changed. The store the cold run left is all the warm run
//! has, so it is read from disk and not from a process that kept it. Over a tree written here; all of ruff is held to the same by the
//! `lint` bench's `--replay` run, and the memory a warm run over ruff peaks at by its `--memory` run (`warm-peak-megabytes`).

#[cfg(target_os = "linux")]
use crate::cache::{function, write};
#[cfg(target_os = "linux")]
use crate::{counting, unbounded};
#[cfg(target_os = "linux")]
use ryo_core::inbound::use_cases::lint::types::Rule;

/// The argument that names the directory a child process runs the warm run over, and so marks the process as the child.
#[cfg(target_os = "linux")]
const WARM_DIRECTORY: &str = "warm-peak-directory=";

#[cfg(target_os = "linux")]
#[test]
fn a_warm_unchanged_run_reads_nothing_in_a_process_of_its_own() {
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
        // The child: the warm run alone, so what it reads is its own.
        let warm = unbounded::lint_counting(&directory, rules, crate::base::store_at(&directory))
            .expect("the rules run");
        assert_eq!(warm.reads, 0, "a warm run reads no file");
    } else {
        let root = tempfile::tempdir().expect("a directory");
        write(
            root.path(),
            &[
                ("one/lib.rs", function("copied", 40, "1")),
                ("two/lib.rs", function("copied", 40, "1")),
                ("three/lib.rs", function("alone", 40, "2")),
            ],
        );
        let cold = unbounded::lint_counting(root.path(), rules, crate::base::store_at(root.path()))
            .expect("the rules run");
        assert!(cold.reads > 0, "a cold run reads the files of the tree");
        let child = std::process::Command::new(std::env::current_exe().expect("this test program"))
            .args([
                "--exact",
                "peak::a_warm_unchanged_run_reads_nothing_in_a_process_of_its_own",
                "--nocapture",
                "--test-threads=1",
            ])
            .arg(format!("{WARM_DIRECTORY}{}", root.path().display()))
            .output()
            .expect("the child runs");
        let said = String::from_utf8_lossy(&child.stderr);
        assert!(child.status.success(), "the child failed: {said}");
    }
}
