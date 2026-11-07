//! Proves how a command ended: each `CommandOutcome` and its exit code, and each error's class.

use super::CommandOutcome;

/// A command that did its job ends with exit code 0.
#[test]
fn success_succeeds_with_exit_code_zero() {
    assert_eq!(CommandOutcome::Success.code(), 0);
}

/// A command that refused its input or reached a negative verdict ends with exit code 1.
#[test]
fn failure_refuses_with_exit_code_one() {
    assert_eq!(CommandOutcome::Failure.code(), 1);
}

/// A command that could not run ends with exit code 2.
#[test]
fn error_refuses_with_exit_code_two() {
    assert_eq!(CommandOutcome::Error.code(), 2);
}

/// Each ending has an exit code of its own, so a caller can tell them apart.
#[test]
fn outcomes_succeed_with_distinct_exit_codes() {
    let codes = [CommandOutcome::Success, CommandOutcome::Failure, CommandOutcome::Error].map(CommandOutcome::code);
    assert_eq!(codes, [0, 1, 2]);
}
