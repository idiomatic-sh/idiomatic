//! What a run keeps of a repository for the next, with the real Rust language, the real fact store and the
//! real store file: a warm run reads no file and answers what the cold run did, an edit that leaves what
//! the files show one another as it was reads only the file edited, and the answer after any edit is the
//! one a run that keeps nothing gives.

use crate::{counting, ruff, stores};
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use ryo_core::inbound::use_cases::lint::types::Rule;

/// The most bytes the store of all of ruff may hold after a run: a regression guard at the size measured (70,304,943 bytes,
/// rounded up to 75 MB). The goal is a few MB (five), and this bound is lowered when the store reaches it.
const STORE_BYTES_BOUND: u64 = 75_000_000;

/// The recorded rules that cross files and that stay in one: the first compares functions of one name in two files.
pub(crate) fn rules() -> Vec<Rule> {
    let mut wanted = counting::runnable();
    wanted.retain(|(name, _)| {
        [
            "duplicate-function-bodies",
            "todo-comments",
            "unwrap-outside-tests",
        ]
        .contains(&name.as_str())
    });
    wanted.into_iter().map(|(_, rule)| rule).collect()
}

/// A function of `statements` statements named `name`.
pub(crate) fn function(name: &str, statements: usize, tag: &str) -> String {
    let mut text = format!("pub fn {name}() {{\n");
    for statement in 0..statements {
        writeln!(
            text,
            "    let value_{statement} = {tag} + {statement}; // TODO {statement}"
        )
        .expect("writing to a String cannot fail");
    }
    text.push_str("}\n");
    text
}

pub(crate) fn write(root: &Path, files: &[(&str, String)]) {
    for (path, text) in files {
        let place = root.join(path);
        std::fs::create_dir_all(place.parent().expect("a directory")).expect("a directory");
        std::fs::write(place, text).expect("a file");
    }
}

pub(crate) fn sorted(mut rows: Vec<Vec<String>>) -> Vec<Vec<String>> {
    rows.sort();
    rows
}

/// What a run that keeps nothing gives.
pub(crate) fn uncached(root: &Path) -> counting::Counted {
    counting::lint_counting(
        root,
        rules(),
        Arc::new(ryo_core::outbound::lint_store::factory::memory()),
    )
    .expect("the rules run")
}

#[test]
fn a_warm_run_reads_nothing_and_an_edit_reads_what_it_must() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    let store = crate::base::store_at(root.path());
    write(
        root.path(),
        &[
            ("one/lib.rs", function("copied", 40, "1")),
            ("two/lib.rs", function("copied", 40, "1")),
            ("three/lib.rs", function("alone", 40, "2")),
        ],
    );

    let (cold, cold_complete) = stores::complete(root.path(), rules(), &store, "a cold run");
    assert_eq!(cold.reads, 3, "a cold run reads every file");
    assert!(
        cold.unread.iter().all(|path| {
            !Path::new(path.as_str())
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
        }),
        "a cold run leaves a Rust file unread"
    );
    assert!(
        cold.rows.iter().any(|row| row
            .first()
            .is_some_and(|rule| rule == "duplicate-function-bodies")),
        "the two copies are found across files: {:?}",
        cold.rows
    );

    let (warm, warm_complete) = stores::complete(root.path(), rules(), &store, "a warm run");
    assert_eq!(
        cold_complete.output, warm_complete.output,
        "warm keeps complete ordered output"
    );
    assert_eq!(
        cold_complete.report, warm_complete.report,
        "warm keeps unread reasons and report metadata"
    );
    assert_eq!(warm.reads, 0, "a warm run reads no file");
    assert_eq!(
        sorted(warm.rows.clone()),
        sorted(cold.rows.clone()),
        "and answers what the cold run did"
    );

    // A comment added after the function changes what the file says, not what it shows the other files.
    let mut commented = function("alone", 40, "2");
    commented.push_str("// TODO later\n");
    write(root.path(), &[("three/lib.rs", commented)]);
    let noted =
        counting::lint_counting(root.path(), rules(), Arc::clone(&store)).expect("an edit run");
    assert_eq!(noted.reads, 1, "only the file edited is read");
    assert_eq!(
        sorted(noted.rows.clone()),
        sorted(uncached(root.path()).rows),
        "and the answer is the one a run that keeps nothing gives"
    );

    // One of the copies is edited so that it is not a copy any more: what it shows the others changed, so the cross-file steps are
    // made again, from what the other files handed the last run; only the file edited is read.
    write(root.path(), &[("two/lib.rs", function("copied", 40, "9"))]);
    let edited =
        counting::lint_counting(root.path(), rules(), Arc::clone(&store)).expect("an edit run");
    assert_eq!(edited.reads, 1, "only the file edited is read");
    assert_eq!(
        sorted(edited.rows.clone()),
        sorted(uncached(root.path()).rows),
        "and the answer is the one a run that keeps nothing gives"
    );

    // A file added that copies another is found by the cross-file steps.
    write(root.path(), &[("four/lib.rs", function("alone", 40, "2"))]);
    let added =
        counting::lint_counting(root.path(), rules(), Arc::clone(&store)).expect("an added file");
    assert_eq!(added.reads, 1, "only the file added is read");
    let oracle = uncached(root.path());
    assert_eq!(
        sorted(added.rows),
        sorted(oracle.rows),
        "the new copy is found"
    );

    // A file removed takes its findings with it.
    std::fs::remove_file(root.path().join("four/lib.rs")).expect("a file");
    let removed = counting::lint_counting(root.path(), rules(), store).expect("a removed file");
    assert_eq!(removed.reads, 0, "a file removed reads no file");
    assert_eq!(sorted(removed.rows), sorted(uncached(root.path()).rows));
}

#[test]
fn a_warm_run_leaves_the_store_as_it_was_and_a_removed_file_does_not() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    let store = crate::base::store_at(root.path());
    write(
        root.path(),
        &[
            ("one/lib.rs", function("copied", 40, "1")),
            ("two/lib.rs", function("copied", 40, "1")),
        ],
    );
    let first =
        counting::lint_counting(root.path(), rules(), Arc::clone(&store)).expect("a cold run");
    let written = || {
        std::fs::metadata(root.path().join(".ryo_cache/lint.store"))
            .and_then(|meta| meta.modified())
            .expect("the store")
    };
    let cold = written();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let warm =
        counting::lint_counting(root.path(), rules(), Arc::clone(&store)).expect("a warm run");
    assert_eq!(
        sorted(warm.rows),
        sorted(first.rows),
        "a warm run answers what the cold run did"
    );
    assert!(written() == cold, "nothing changed, so nothing is written");
    std::fs::remove_file(root.path().join("two/lib.rs")).expect("a file");
    let removed = counting::lint_counting(root.path(), rules(), store)
        .expect("a run after a file is removed");
    assert_eq!(removed.reads, 0, "the file left is not read");
    assert!(written() != cold, "a file is gone, so the store says so");
}

#[test]
fn a_store_is_not_used_for_other_rules_and_is_kept_apart_by_root() {
    let _beside = crate::beside();
    let (first, second) = (
        tempfile::tempdir().expect("a directory"),
        tempfile::tempdir().expect("a directory"),
    );
    let store = crate::base::store_at(first.path());
    let other = crate::base::store_at(second.path());
    write(first.path(), &[("lib.rs", function("one", 5, "1"))]);
    write(second.path(), &[("lib.rs", function("two", 5, "1"))]);
    assert_eq!(
        counting::lint_counting(first.path(), rules(), Arc::clone(&store))
            .expect("a run")
            .reads,
        1,
        "the first run of a root reads its one file"
    );
    assert_eq!(
        counting::lint_counting(second.path(), rules(), other)
            .expect("a run")
            .reads,
        1,
        "another root has no store"
    );
    let fewer: Vec<Rule> = rules().into_iter().take(1).collect();
    assert_eq!(
        counting::lint_counting(first.path(), fewer, Arc::clone(&store))
            .expect("a run")
            .reads,
        1,
        "other rules read every file"
    );
    assert_eq!(
        counting::lint_counting(first.path(), rules(), store)
            .expect("a run")
            .reads,
        1,
        "and the store they left is the store of those rules"
    );
}

#[test]
fn every_recorded_rule_gives_over_ruff_the_rows_a_warm_run_gives_again_from_a_small_store() {
    let _beside = crate::beside();
    let copy = ruff::ruff_copy();
    let rules: Vec<Rule> = counting::runnable()
        .into_iter()
        .map(|(_, rule)| rule)
        .collect();
    let store = || crate::base::store_at(copy.path());
    let cold = counting::lint_counting(copy.path(), rules.clone(), store()).expect("the rules run");
    // The store keeps the findings, a digest of each file and rule, and what each file hands the steps that read across files.
    let size = std::fs::metadata(copy.path().join(".ryo_cache/lint.store"))
        .expect("the store")
        .len();
    assert!(
        size <= STORE_BYTES_BOUND,
        "the store of ruff stays within {STORE_BYTES_BOUND} bytes: {size} bytes"
    );
    let warm = counting::lint_counting(copy.path(), rules, store()).expect("the rules run");
    assert_eq!(warm.reads, 0, "a warm run reads no file");
    let by_rule = |rows: &[Vec<String>]| {
        let mut counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for row in rows {
            *counts
                .entry(row.first().cloned().unwrap_or_default())
                .or_default() += 1;
        }
        counts
    };
    assert_eq!(
        by_rule(&warm.rows),
        by_rule(&cold.rows),
        "each rule gives as many rows warm as cold"
    );
    assert_eq!(sorted(warm.rows), sorted(cold.rows));
}

#[test]
fn files_added_and_removed_before_others_move_what_the_others_handed_with_them() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    let store = crate::base::store_at(root.path());
    // Two pairs of copies; a file added or removed before them in the order of the paths moves the number of each file after it.
    write(
        root.path(),
        &[
            ("b/lib.rs", function("first", 40, "1")),
            ("d/lib.rs", function("first", 40, "1")),
            ("f/lib.rs", function("second", 40, "2")),
            ("h/lib.rs", function("second", 40, "2")),
        ],
    );
    let as_kept_nothing = |step: &str, run: counting::Counted| {
        assert_eq!(
            sorted(run.rows),
            sorted(uncached(root.path()).rows),
            "{step}: the answer is the one a run that keeps nothing gives"
        );
    };
    let run =
        |step: &str| counting::lint_counting(root.path(), rules(), Arc::clone(&store)).expect(step);
    as_kept_nothing("a cold run", run("a cold run"));
    // A file too small to hand the cross-file steps anything moves every other file one number on.
    write(root.path(), &[("a/lib.rs", function("tiny", 1, "3"))]);
    as_kept_nothing("a small file added first", run("a small file added first"));
    // One of the second pair is no copy any more: the cross-file steps are made again from what the others handed.
    write(root.path(), &[("f/lib.rs", function("second", 40, "9"))]);
    as_kept_nothing("a copy edited", run("a copy edited"));
    // The small file goes and a third copy of the first pair comes between the others: the numbers move again.
    std::fs::remove_file(root.path().join("a/lib.rs")).expect("a file");
    write(root.path(), &[("c/lib.rs", function("first", 40, "1"))]);
    as_kept_nothing(
        "a file removed and one added",
        run("a file removed and one added"),
    );
    write(root.path(), &[("h/lib.rs", function("second", 40, "9"))]);
    as_kept_nothing("the other copy edited", run("the other copy edited"));
    let warm = run("a warm run");
    assert_eq!(warm.reads, 0, "a warm run reads no file");
    as_kept_nothing("a warm run", warm);
}

#[test]
fn an_edit_hands_the_store_the_words_of_the_file_read_alone() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    let counting = stores::counting_at(root.path());
    write(
        root.path(),
        &[
            ("one/lib.rs", function("copied", 40, "1")),
            ("two/lib.rs", function("copied", 40, "1")),
            ("three/lib.rs", function("alone", 40, "2")),
        ],
    );
    let cold = counting::lint_counting(
        root.path(),
        rules(),
        Arc::<stores::CountingStore>::clone(&counting),
    )
    .expect("a cold run");
    assert_eq!(cold.reads, 3, "a cold run reads every file");
    assert_eq!(
        counting.handed_files.load(Ordering::Relaxed),
        3,
        "a cold run hands the store the words of every file"
    );
    // One copy is renamed, so it is no copy any more: the cross-file steps are made again, and the store is handed the words of that
    // file alone; the words of the two used again stay where it keeps them.
    write(root.path(), &[("two/lib.rs", function("renamed", 40, "1"))]);
    let edited = counting::lint_counting(
        root.path(),
        rules(),
        Arc::<stores::CountingStore>::clone(&counting),
    )
    .expect("an edit run");
    assert_eq!(edited.reads, 1, "only the file edited is read");
    assert_eq!(
        (
            counting.handed_files.load(Ordering::Relaxed),
            counting.files_again.load(Ordering::Relaxed)
        ),
        (1, 2),
        "the store is handed the words of the file read, and moves those of the two used again"
    );
    assert_eq!(
        sorted(edited.rows),
        sorted(uncached(root.path()).rows),
        "and the answer is the one a run that keeps nothing gives"
    );
    let warm = counting::lint_counting(root.path(), rules(), counting).expect("a warm run");
    assert_eq!(
        sorted(warm.rows),
        sorted(uncached(root.path()).rows),
        "the words it kept where they were answer the next run"
    );
}

#[test]
fn a_run_whose_store_keeps_nothing_answers_what_one_that_keeps_does() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    write(
        root.path(),
        &[
            ("one/lib.rs", function("copied", 40, "1")),
            ("two/lib.rs", function("copied", 40, "1")),
            ("three/lib.rs", function("alone", 40, "2")),
        ],
    );
    let plain = counting::lint_counting(root.path(), rules(), stores::nothing_kept())
        .expect("a run that keeps nothing");
    assert_eq!(plain.reads, 3, "every file is read");
    assert_eq!(
        sorted(plain.rows),
        sorted(uncached(root.path()).rows),
        "and the findings are those of a run that keeps them"
    );
}
