//! A fixed answer standing in for the machine.
use crate::outbound::host::interface::HostPort;
use ryo_benchmark_contracts::domain::hosts::types::{
    HostFacts, HostFamily, HostIsolation, HostKernel,
};

/// Deterministic host substitute.
#[derive(Clone, Copy, Debug)]
pub struct HostFake {
    /// What every question is answered with.
    pub facts: HostFacts,
}

impl Default for HostFake {
    /// A native Unix kernel outside a container: a host whose file permissions are honoured.
    fn default() -> Self {
        Self {
            facts: HostFacts {
                family: HostFamily::Unix,
                kernel: HostKernel::Native,
                isolation: HostIsolation::Bare,
            },
        }
    }
}

impl HostPort for HostFake {
    fn facts(&self) -> HostFacts {
        self.facts
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
