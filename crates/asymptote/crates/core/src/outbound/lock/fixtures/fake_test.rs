//! The `lock` port's fake runs the port's contract.

use super::super::contract::check;
use super::LockFake;

#[test]
fn succeeds() {
    check(&LockFake::default());
}
