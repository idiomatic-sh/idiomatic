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
    /// The processors the process may run on: four, the count every bench is measured on, unless a row says otherwise.
    pub processors: crate::outbound::host::types::ProcessorCount,
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
            processors: crate::outbound::host::types::ProcessorCount(
                std::num::NonZeroUsize::new(
                    ryo_benchmark_contracts::domain::measurements::constants::MEASURED_WORKERS,
                )
                .unwrap_or(std::num::NonZeroUsize::MIN),
            ),
        }
    }
}

impl HostPort for HostFake {
    fn facts(&self) -> HostFacts {
        self.facts
    }
    fn processors(&self) -> crate::outbound::host::types::ProcessorCount {
        self.processors
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
