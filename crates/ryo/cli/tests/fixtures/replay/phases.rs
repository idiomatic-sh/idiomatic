//! What only the bench adds: the runs of the cache rows and of ruff as it is.

use super::base::lint_over;
use super::counting::lint_observing;
use std::path::Path;
use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::types::Rule;

/// One run of the cache rows: what it was, how long it took, how many files were read, and how many findings it gave.
pub(crate) struct Phase {
    /// SHA-256 of complete emitted output and report, computed after stopping the timers.
    pub output_digest: String,
    /// Cold, warm, or after an edit.
    pub label: &'static str,
    /// Wall time in milliseconds.
    pub wall: u128,
    /// CPU time of the process in milliseconds.
    pub cpu: u128,
    /// How many files the language read.
    pub reads: usize,
    /// How many times the language read the project.
    pub projects: usize,
    /// How many findings.
    pub rows: usize,
    /// How many Rust files no language read: none, for a run over ruff.
    pub unread_rust: usize,
}

/// Appends a function to one file of the copy of ruff.
///
/// # Panics
///
/// When the file cannot be edited.
pub(crate) fn edit_ruff_copy(copy: &Path) {
    let edited = copy.join("crates/ruff_python_ast/src/lib.rs");
    let mut text = std::fs::read_to_string(&edited).expect("a file of ruff to edit");
    text.push_str("\npub fn edited_for_the_cache_row() {}\n");
    std::fs::write(&edited, text).expect("the edit");
}

/// Replaces `from` with `to`, of the same length, once in the file `path` of the copy of ruff: no other place of the file moves.
///
/// # Panics
///
/// When the file cannot be edited, or does not hold `from`.
fn replace_in_ruff_copy(copy: &Path, path: &str, from: &str, to: &str) {
    let edited = copy.join(path);
    let text = std::fs::read_to_string(&edited).expect("a file of ruff to edit");
    assert!(text.contains(from), "{path} holds `{from}`");
    std::fs::write(&edited, text.replacen(from, to, 1)).expect("the edit");
}

/// Edits the body of one function of the copy of ruff: a number it computes with, so nothing it exports changes.
///
/// # Panics
///
/// When the file cannot be edited.
pub(crate) fn edit_ruff_body(copy: &Path) {
    replace_in_ruff_copy(
        copy,
        "crates/ruff_python_ast/src/find_node.rs",
        "checked_sub(2)",
        "checked_sub(3)",
    );
}

/// Renames a function one file of the copy of ruff exports, to a name of the same length.
///
/// # Panics
///
/// When the file cannot be edited.
pub(crate) fn rename_ruff_export(copy: &Path) {
    replace_in_ruff_copy(
        copy,
        "crates/ruff_python_ast/src/lib.rs",
        "pub const fn is_ipynb(self)",
        "pub const fn is_ipnb_(self)",
    );
}

/// Lints the copy of ruff once with `rules`, keeping its store in the copy (or nothing), and times it.
///
/// # Panics
///
/// When a run is refused.
pub(crate) fn ruff_phase(copy: &Path, rules: Vec<Rule>, keep: bool, label: &'static str) -> Phase {
    use sha2::Digest as _;
    let kept: Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort> = if keep {
        crate::base::store_at(copy)
    } else {
        // No store, as the command line runs without a cache.
        Arc::new(ryo_core::outbound::lint_store::factory::discard())
    };
    let started = std::time::Instant::now();
    let cpu = cpu_time::ProcessTime::now();
    // The complete output and report are kept as they are, and written as text and digested only after the timers stop:
    // writing them is the bench's work, not the run's.
    let (run, output, report) = lint_observing(copy, rules, kept, 0, |counted, output, report| {
        (counted, output, report)
    })
    .expect("the rules run over the copy");
    let wall = started.elapsed().as_millis();
    let cpu = cpu.elapsed().as_millis();
    let output_digest = format!(
        "{:x}",
        sha2::Sha256::digest(format!("{output:?}\n{report:?}").as_bytes())
    );
    Phase {
        output_digest,
        label,
        wall,
        cpu,
        reads: run.reads,
        projects: run.projects,
        rows: run.rows.len(),
        unread_rust: run
            .unread
            .iter()
            .filter(|path| {
                Path::new(path.as_str())
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
            })
            .count(),
    }
}

/// Lints ruff cold with `rules` and answers how many rows they gave.
///
/// # Panics
///
/// When the rules do not run.
pub(crate) fn lint_ruff(rules: Vec<Rule>) -> usize {
    lint_over(&super::base::snapshot::ruff(), rules)
        .expect("the rules run over ruff")
        .0
        .len()
}

/// Reads every file under `root` once, in one sequential pass, and keeps nothing: afterwards the operating system's page cache holds the
/// target, the same in every round, so a round measures the engine and not whether the disk was asked. "Cold" in the ratchet means no
/// store of ryo's own, not a cold disk. Called before the timer starts.
pub(crate) fn pre_read(root: &Path) {
    let mut buffer = Vec::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        let entries = std::fs::read_dir(&directory).expect("a directory of the target is read");
        let mut entries: Vec<_> = entries
            .map(|entry| entry.expect("an entry of a directory of the target is read"))
            .collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                directories.push(entry.path());
                continue;
            }
            let Ok(mut file) = std::fs::File::open(entry.path()) else {
                continue;
            };
            buffer.clear();
            // The read is for the page cache alone: a file that cannot be read is left for the run itself to find.
            drop(std::io::Read::read_to_end(&mut file, &mut buffer));
        }
    }
}
