//! What the replay answers, the recordings replayed as a person reads them: each answer begins as it is said here.

use super::answers::answer;

/// Asserts that the replay of `input` answers with something that begins with `expected`.
fn answers(input: &str, expected: &str) {
    let said = answer(input);
    assert!(
        said.starts_with(expected),
        "the replay answered `{said}`, not `{expected}`"
    );
}

#[test]
fn recorded_recognition_gives_the_recorded_unread_files() {
    answers("recognition", "the unread files are the recorded ones");
}

#[test]
fn row_that_differs_is_reported_with_its_plan_id() {
    answers(
        "a_wrong_row",
        "rule `deep-nesting` (plan `query-8`) differs at row 1",
    );
}

#[test]
fn the_files_that_say_why_something_differs_name_only_recorded_rules_and_real_features() {
    answers("names", "every name is a recorded rule or a feature");
}
