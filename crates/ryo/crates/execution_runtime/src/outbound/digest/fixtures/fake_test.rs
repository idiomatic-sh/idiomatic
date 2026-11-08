//! The `digest` port's fake runs the port's contract.

use super::super::contract::check;
use super::DigestFake;

#[test]
fn succeeds() {
    check(&DigestFake);
}
