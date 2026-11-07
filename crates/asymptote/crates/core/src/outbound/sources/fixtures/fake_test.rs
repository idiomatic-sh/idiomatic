//! The `sources` port's fake runs the port's contract.

use super::super::contract::check;
use super::SourcesFake;

#[test]
fn succeeds() {
    check(&SourcesFake::holding(&[("src/a.rs", "fn a() {}\n")]));
}
