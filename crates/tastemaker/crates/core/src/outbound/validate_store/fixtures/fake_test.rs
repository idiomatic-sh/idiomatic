//! The `validate_store` port's fake runs the port's contract.

use super::super::contract::check;
use super::ValidateStoreFake;

#[test]
fn succeeds() {
    check(&ValidateStoreFake::default());
}
