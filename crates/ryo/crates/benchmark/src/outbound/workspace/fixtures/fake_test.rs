//! The `workspace` port's fake runs the port's contract.

use super::super::contract::check;
use super::WorkspaceFake;

#[test]
fn succeeds() {
    check(&WorkspaceFake);
}
