//! The `matching` port's fake runs the port's contract.

use super::super::contract::check;
use super::MatchingFake;

#[test]
fn succeeds() {
    check(&MatchingFake);
}
