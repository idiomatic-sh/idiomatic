//! The `context` port's fake runs the port's contract.
use super::super::contract::check;
use super::ContextFake;

#[test]
fn succeeds() {
    check(&ContextFake);
}
