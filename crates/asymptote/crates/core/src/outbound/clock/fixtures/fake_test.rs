//! The `clock` port's fake runs the port's contract.

use super::super::contract::check;
use super::ClockFake;

#[test]
fn succeeds() {
    check(&ClockFake);
}
