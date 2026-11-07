//! The `pattern_matching` port's fake runs the port's contract.

use super::super::contract::check;
use super::PatternMatchingFake;

#[test]
fn succeeds() {
    check(&PatternMatchingFake);
}
