//! The `publication` port's fake runs the port's contract.

use super::super::contract::check;
use super::PublicationFake;

#[test]
fn succeeds() {
    check(&PublicationFake);
}
