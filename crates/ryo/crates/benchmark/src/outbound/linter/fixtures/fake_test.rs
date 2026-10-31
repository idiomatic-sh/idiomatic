//! The `linter` port's fake runs the port's contract.

use super::super::contract::check;
use super::LinterFake;

#[test]
fn succeeds() {
    check(&LinterFake);
}
