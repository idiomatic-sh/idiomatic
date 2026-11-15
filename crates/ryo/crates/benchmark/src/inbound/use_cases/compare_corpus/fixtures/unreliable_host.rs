//! A parity use case on a host whose file permissions are unreliable, whose rules' settings profile requires file observations.
use crate::inbound::use_cases::compare_corpus::use_case::CompareCorpusUseCaseImpl;
use crate::outbound::{
    context::fixtures::fake::ContextFake, evidence::fixtures::fake::EvidenceFake,
    execution::fixtures::fake::ExecutionFake, host::fixtures::fake::HostFake,
    linter::fixtures::fake::LinterFake, suite::fixtures::fake::SuiteFake,
    suite::fixtures::samples::tool,
};
use ryo_benchmark_contracts::domain::hosts::types::{
    HostFacts, HostFamily, HostIsolation, HostKernel,
};
use ryo_benchmark_contracts::domain::workloads::types::{FileObservation, Tool};
use std::sync::Arc;

/// The use case on a Microsoft kernel outside a container, over a suite whose profile requires `PosixExecutable`.
#[must_use]
pub fn use_case() -> CompareCorpusUseCaseImpl {
    CompareCorpusUseCaseImpl::new(
        Arc::new(LinterFake { diagnostics: Vec::new(), arguments: Vec::new() }),
        Arc::new(LinterFake { diagnostics: Vec::new(), arguments: Vec::new() }),
        Arc::new(ExecutionFake { status: 0, ..ExecutionFake::default() }),
        Arc::new(ContextFake),
        Arc::new(SuiteFake {
            profile: Some(Tool { file_observation: FileObservation::PosixExecutable, ..tool() }),
            ..SuiteFake::default()
        }),
        Arc::new(EvidenceFake),
        Arc::new(crate::outbound::discovery::fixtures::fake::DiscoveryFake { files: Vec::new() }),
        Arc::new(crate::outbound::workspace::fixtures::fake::WorkspaceFake),
        Arc::new(HostFake {
            facts: HostFacts {
                family: HostFamily::Unix,
                kernel: HostKernel::Microsoft,
                isolation: HostIsolation::Bare,
            },
        }),
    )
}
