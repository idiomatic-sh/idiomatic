//! The `check_store` port's fake runs the port's contract.

use super::super::contract::check;
use super::CheckStoreFake;

#[test]
fn succeeds() {
    check(&CheckStoreFake::default());
}
