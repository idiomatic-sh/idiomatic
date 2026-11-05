//! The `execution` port's fake runs the port's contract.

use super::super::contract::check;
use super::ExecutionFake;

#[test]
fn succeeds() {
    check(&ExecutionFake::default());
}
