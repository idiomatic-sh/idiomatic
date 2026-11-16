//! The `--replay` run: what the recorded rules answer over ruff and the other pinned targets, each answer held to what was recorded
//! of it, too big a run for the test suite. Each check is the test it was, by the same name, with the same assertion and the same
//! failure: every check runs, each failure is said on stderr as it happens, and the run fails when one did. Asymptote's
//! `bench-replay` tool runs it in the `bench-correctness` group; the replay's own suite holds the same answers over trees it writes.

use crate::{base, counting, handovers, replayed, ruff};
use ryo_core::inbound::use_cases::lint::types::Rule;

/// The argument that names the directory a child process runs the warm run over, and so marks the process as the child.
pub(crate) const WARM_DIRECTORY: &str = "--warm-directory";

/// Every check of the run, by the name of the test it was.
const CHECKS: [(&str, fn()); 10] = [
    (
        "behavior::every_recorded_rule_gives_the_recorded_rows",
        every_recorded_rule_gives_the_recorded_rows,
    ),
    (
        "behavior::recorded_recognition_gives_the_recorded_unread_files",
        recorded_recognition_gives_the_recorded_unread_files,
    ),
    (
        "behavior::ruff_findings_equal_the_expected_findings",
        ruff_findings_equal_the_expected_findings,
    ),
    (
        "behavior::ruff_rules_feed_every_catalog_feature_a_thousand_rows",
        ruff_rules_feed_every_catalog_feature_a_thousand_rows,
    ),
    (
        "behavior::row_that_differs_is_reported_with_its_plan_id",
        row_that_differs_is_reported_with_its_plan_id,
    ),
    (
        "behavior::what_ryo_could_not_decide_over_ruff_is_the_golden_or_a_recorded_difference",
        what_ryo_could_not_decide_over_ruff_is_the_golden_or_a_recorded_difference,
    ),
    (
        "behavior::the_files_that_say_why_something_differs_name_only_recorded_rules_and_real_features",
        the_files_that_say_why_something_differs_name_only_recorded_rules_and_real_features,
    ),
    (
        "handover::no_file_of_ruff_hands_a_rule_more_than_four_rows_for_each_row_of_its_tree",
        no_file_of_ruff_hands_a_rule_more_than_four_rows_for_each_row_of_its_tree,
    ),
    (
        "peak::a_warm_unchanged_run_over_ruff_reads_nothing_in_a_process_of_its_own",
        a_warm_unchanged_run_over_ruff_reads_nothing_in_a_process_of_its_own,
    ),
    (
        "manifests_cargo::ryo_says_what_cargo_says_of_the_manifests_of_every_target",
        ryo_says_what_cargo_says_of_the_manifests_of_every_target,
    ),
];

/// Runs every check, each after the last, and fails when one failed, naming each that did.
///
/// # Panics
///
/// When a check fails.
pub(crate) fn run() {
    let mut failed = Vec::new();
    for (name, check) in CHECKS {
        eprintln!("replay {name} ...");
        // A failed check panics with its assertion's message, which the panic hook says on stderr: the others still run.
        if std::panic::catch_unwind(check).is_err() {
            failed.push(name);
        }
        eprintln!(
            "replay {name} {}",
            if failed.last() == Some(&name) {
                "FAILED"
            } else {
                "ok"
            }
        );
    }
    assert!(
        failed.is_empty(),
        "{} of {} replay checks failed: {failed:#?}",
        failed.len(),
        CHECKS.len()
    );
}

/// Asserts that the replay of `input` answers with something that begins with `expected`.
fn answers(input: &str, expected: &str) {
    let said = replayed::answer(input);
    assert!(
        said.starts_with(expected),
        "the replay answered `{said}`, not `{expected}`"
    );
}

/// Every live recorded rule gives over ruff the rows its recording holds.
fn every_recorded_rule_gives_the_recorded_rows() {
    answers("recorded_rules", "every rule gives its rows");
}

/// The files ryo leaves unread over ruff are the recorded ones.
fn recorded_recognition_gives_the_recorded_unread_files() {
    answers("recognition", "the unread files are the recorded ones");
}

/// Each live rule's distinct rows over ruff are as many as the golden findings expect.
fn ruff_findings_equal_the_expected_findings() {
    answers("findings", "the findings are the expected ones");
}

/// Every feature of the language is fed a thousand rows over ruff, or `fixtures/unfed.toml` says why not.
fn ruff_rules_feed_every_catalog_feature_a_thousand_rows() {
    answers("features", "every feature is fed");
}

/// A row that differs from the recording is said with its rule and its plan.
fn row_that_differs_is_reported_with_its_plan_id() {
    answers(
        "a_wrong_row",
        "rule `deep-nesting` (plan `query-8`) differs at row 1",
    );
}

/// What ryo could not decide over ruff is the golden's, or a recorded difference says why not.
fn what_ryo_could_not_decide_over_ruff_is_the_golden_or_a_recorded_difference() {
    answers(
        "unknowns",
        "every unknown is the golden's or a recorded difference",
    );
}

/// The files that say why something differs name only recorded rules and features of the language.
fn the_files_that_say_why_something_differs_name_only_recorded_rules_and_real_features() {
    answers("names", "every name is a recorded rule or a feature");
}

/// No file of ruff hands a rule more than four rows for each row of its tree.
fn no_file_of_ruff_hands_a_rule_more_than_four_rows_for_each_row_of_its_tree() {
    let copy = ruff::ruff_copy();
    let handovers = handovers::handovers_over(copy.path());
    // A run that counted nothing would hold every bound: the run is held to have counted a handover for most of ruff's files.
    assert!(
        handovers.len() > 10_000,
        "the counting run reports a handover for each file that handed something, not {}",
        handovers.len()
    );
    handovers::within_the_bound(handovers);
}

/// Every recorded rule, as a warm run is asked for them.
fn rules() -> Vec<Rule> {
    counting::runnable()
        .into_iter()
        .map(|(_, rule)| rule)
        .collect()
}

/// The child of [`a_warm_unchanged_run_over_ruff_reads_nothing_in_a_process_of_its_own`]: the warm run over `directory` alone, so what
/// it reads is its own.
///
/// # Panics
///
/// When the warm run reads a file.
pub(crate) fn warm_child(directory: &std::path::Path) {
    let warm = counting::lint_with_workers(directory, rules(), base::store_at(directory), 0)
        .expect("the rules run");
    assert_eq!(warm.reads, 0, "a warm run reads no file");
}

/// A warm run over an unchanged ruff, in a process of its own, reads no file.
fn a_warm_unchanged_run_over_ruff_reads_nothing_in_a_process_of_its_own() {
    let copy = ruff::ruff_copy();
    let _cold = counting::lint_with_workers(copy.path(), rules(), base::store_at(copy.path()), 0)
        .expect("the rules run");
    let child = std::process::Command::new(std::env::current_exe().expect("this bench program"))
        .arg(WARM_DIRECTORY)
        .arg(copy.path())
        .output()
        .expect("the child runs");
    let said = String::from_utf8_lossy(&child.stderr);
    assert!(child.status.success(), "the child failed: {said}");
}

/// Ryo says what Cargo says of the manifests of every pinned Rust target.
fn ryo_says_what_cargo_says_of_the_manifests_of_every_target() {
    // Cargo, run with its environment cleared and an empty `CARGO_HOME`: the tools it needs to be found are all it is given.
    let home = tempfile::tempdir().expect("a directory").keep();
    let home = home.to_string_lossy().into_owned();
    let mut found_differences = Vec::new();
    for name in ["ruff", "serde", "tokio"] {
        let root =
            std::fs::canonicalize(crate::snapshot::target(name)).expect("the target is there");
        let (differences, _said) = crate::cargo::cargo_differences(name, &root, &home);
        found_differences.extend(differences);
    }
    assert!(found_differences.is_empty(), "{found_differences:#?}");
}
