//! The `profile` port's fake runs the port's contract.

use super::super::contract::check;
use super::ProfileFake;

#[test]
fn succeeds() {
    check(&ProfileFake::<u8>::default());
}
