//! The pure decision from host facts to file-permission applicability.
use ryo_benchmark_contracts::domain::hosts::types::{
    FilePermissions, HostFacts, HostFamily, HostIsolation, HostKernel,
};

/// Whether the host's file permissions can be trusted: never off Unix, and not under a Microsoft kernel outside a container.
#[must_use]
pub(crate) fn permissions(facts: &HostFacts) -> FilePermissions {
    match (facts.family, facts.kernel, facts.isolation) {
        (HostFamily::Other, _, _)
        | (HostFamily::Unix, HostKernel::Microsoft, HostIsolation::Bare) => {
            FilePermissions::Unreliable
        }
        (HostFamily::Unix, HostKernel::Microsoft, HostIsolation::Container)
        | (HostFamily::Unix, HostKernel::Native, _) => FilePermissions::Honoured,
    }
}
