//! The `observer` port's fake runs the port's contract.

use super::super::contract::check;
use super::ObserverFake;

#[test]
fn succeeds() {
    check(&ObserverFake::default());
}
