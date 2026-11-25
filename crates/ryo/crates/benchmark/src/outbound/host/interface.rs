//! What the `host` port asks of the machine, in the shared vocabulary; it never takes configuration.
use benchmark_contracts::domain::hosts::types::HostFacts;

/// What every `host` adapter implements: the observation of this machine.
pub trait HostPort: Send + Sync {
    /// The kernel, container and file-permission family this process runs on.
    fn facts(&self) -> HostFacts;
    /// How many processors this process may run on: a bench refuses any count but four.
    fn processors(&self) -> super::types::ProcessorCount;
}
