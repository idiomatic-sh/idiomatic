//! The `conditions` port's fake runs the port's contract.

use super::super::contract::check;
use super::ConditionsFake;

#[test]
fn succeeds() {
    check(&ConditionsFake);
}
