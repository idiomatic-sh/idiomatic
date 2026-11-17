//! The `host` port's fake runs the port's contract.
use super::super::contract::check;
use super::HostFake;
use ryo_benchmark_contracts::domain::hosts::types::{
    HostFacts, HostFamily, HostIsolation, HostKernel,
};

#[test]
fn succeeds() {
    check(&HostFake {
        facts: HostFacts {
            family: HostFamily::Unix,
            kernel: HostKernel::Native,
            isolation: HostIsolation::Bare,
        },
        ..HostFake::default()
    });
}
