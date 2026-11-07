//! The `lint_store` port's fake runs the port's contract.

use super::super::contract::check;
use super::LintStoreFake;

#[test]
fn succeeds() {
    check(&|| Box::new(LintStoreFake::default()));
}
