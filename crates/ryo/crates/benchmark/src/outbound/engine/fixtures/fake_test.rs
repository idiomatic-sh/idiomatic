//! The engine fake preserves the port's refusal contract.
use super::super::contract::check;
use super::EngineFake;
#[test]
fn succeeds() {
    check(&EngineFake { refuses: false });
}
