//! The Linux meter runs the port's contract.
use super::adapter::ProcessMeterLinuxAdapter;
use crate::outbound::process_meter::fixtures::contract::check;

#[test]
fn succeeds() {
    check(&ProcessMeterLinuxAdapter);
}
