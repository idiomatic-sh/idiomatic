//! The `scheduler` port's fake runs the port's contract.

use super::super::contract::check;
use super::SchedulerFake;

#[test]
fn succeeds() {
    check(&SchedulerFake);
}
