//! Request-graph identities and ordered dependency contracts over the operators the application prepares.
use super::fixtures::{CASES, subject};
use ryo_values::cases::{Word, run};

/// Run every row that resolves.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
