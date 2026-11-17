//! The `process_meter` port's fake runs the port's contract.
use super::super::contract::check;
use super::ProcessMeterFake;

#[test]
fn succeeds() {
    check(&ProcessMeterFake::default());
}
