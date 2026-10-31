//! The `suite` port's fake runs the port's contract.

use super::super::contract::check;
use super::SuiteFake;

#[test]
fn succeeds() {
    check(&SuiteFake);
}
