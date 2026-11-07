//! The `inventory` port's fake runs the port's contract.
use super::super::contract::check;
use super::InventoryFake;

#[test]
fn succeeds() {
    check(&InventoryFake::default());
}
