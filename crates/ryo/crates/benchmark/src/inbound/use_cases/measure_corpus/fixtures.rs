//! One certified corpus measured over the fakes of every boundary it calls.
#![expect(
    clippy::expect_used,
    reason = "fixture rule identifiers and sealed values are test preconditions"
)]

use super::{
    types::{
        CorpusLanguage, CorpusMeasurement, ProfileLabel, SampleLabel, SourceProfile, SourceSuffix,
        ToolTitle,
    },
    use_case::MeasureCorpusUseCaseImpl,
};
use crate::domain::measurements::types::LanguageTitle;
use crate::domain::suite::types::{audit::value::Audit, certificate::value::Certificate};
use crate::outbound::{
    context::fixtures::fake::ContextFake, discovery::fixtures::fake::DiscoveryFake,
    evidence::fixtures::fake::EvidenceFake, execution::fixtures::fake::ExecutionFake,
    linter::fixtures::fake::LinterFake, suite::fixtures::fake::SuiteFake,
    workspace::fixtures::fake::WorkspaceFake,
};
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::measurements::types::{
    ExecutionProfile, Invocation, MeasurementMode,
};
use ryo_benchmark_contracts::domain::requests::types::{
    BenchmarkCoverage, CertificationCases, CohortSelection, ContextSeal, FileMetadataDeclaration,
    InputDeclaration, RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
};
use ryo_benchmark_contracts::domain::workloads::types::{Project, RuleContextProfile};
use std::sync::Arc;

/// The bench directory whose settings profiles configure the fixture rules.
pub const BENCH: &str = "bench";

/// The certificate both fixture contestants hold, covering the `covered` rules.
fn certificate(covered: &[&str]) -> Certificate {
    let request = RunRequest {
        performance: Default::default(),
        selected_rules: CohortSelection::Frozen,
        rule_observations: InputDeclaration::Undeclared,
        rule_context: RuleContextPreparation::Unconfigured,
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: StagedSources::Unlisted,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context_manifest: ContextSeal::Unsealed,
        root: ".".into(),
        files: vec![ryo::linter::RelativePath("file".into())],
        state: "cache".into(),
    };
    let profile = Invocation {
        executable: "fixture".into(),
        arguments: Vec::new(),
        environment: std::collections::BTreeMap::new(),
        directory: ".".into(),
    };
    Certificate {
        case_source: CertificationCases::Profile,
        coverage: Audit {
            implemented: covered
                .iter()
                .map(|rule| ryo::linter::RuleId::try_from(*rule).expect("rule identifier"))
                .collect(),
            missing: vec![],
        },
        paths: vec![],
        fingerprint:
            ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest(
                "sealed".into(),
            ),
        contestants: ["first".into(), "second".into()],
        probe: request,
        profiles: [profile.clone(), profile],
        execution: ExecutionProfile::Wall,
        cases: ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount(2),
    }
}

/// The use case wired to the fakes of every boundary it calls.
#[must_use]
pub fn use_case() -> MeasureCorpusUseCaseImpl {
    MeasureCorpusUseCaseImpl::new(
        Arc::new(LinterFake::default()),
        Arc::new(LinterFake::default()),
        Arc::new(DiscoveryFake::default()),
        Arc::new(ExecutionFake::default()),
        Arc::new(SuiteFake {
            profile: Some(crate::outbound::suite::fixtures::samples::tool()),
            ..SuiteFake::default()
        }),
        Arc::new(ContextFake),
        Arc::new(EvidenceFake),
        Arc::new(WorkspaceFake),
        Arc::new(crate::outbound::host::fixtures::fake::HostFake::default()),
    )
}

/// The use case on a Microsoft kernel outside a container, whose suite's settings profile requires executable bits.
#[must_use]
pub fn unreliable_host_use_case() -> MeasureCorpusUseCaseImpl {
    MeasureCorpusUseCaseImpl::new(
        Arc::new(LinterFake::default()),
        Arc::new(LinterFake::default()),
        Arc::new(DiscoveryFake::default()),
        Arc::new(ExecutionFake::default()),
        Arc::new(SuiteFake {
            profile: Some(ryo_benchmark_contracts::domain::workloads::types::Tool {
                file_observation:
                    ryo_benchmark_contracts::domain::workloads::types::FileObservation::PosixExecutable,
                ..crate::outbound::suite::fixtures::samples::tool()
            }),
            ..SuiteFake::default()
        }),
        Arc::new(ContextFake),
        Arc::new(EvidenceFake),
        Arc::new(WorkspaceFake),
        Arc::new(crate::outbound::host::fixtures::fake::HostFake {
            facts: ryo_benchmark_contracts::domain::hosts::types::HostFacts {
                family: ryo_benchmark_contracts::domain::hosts::types::HostFamily::Unix,
                kernel: ryo_benchmark_contracts::domain::hosts::types::HostKernel::Microsoft,
                isolation: ryo_benchmark_contracts::domain::hosts::types::HostIsolation::Bare,
            },
        }),
    )
}

/// Hand `operate` a corpus of `projects` under `scope`; each project is pinned to a branch name, which the workspace refuses.
///
/// # Panics
/// Never: the rule identifier is a valid literal.
pub fn with_corpus<R>(
    scope: BenchmarkCoverage,
    projects: usize,
    operate: impl FnOnce(&CorpusMeasurement<'_>) -> R,
) -> R {
    with_cohort(scope, projects, &["A"], &CohortSelection::Frozen, operate)
}

/// Hand `operate` a corpus of `projects` under `scope`, certified for the `covered` rules and measuring `cohort`.
///
/// # Panics
/// When a covered rule is not a valid rule identifier.
pub fn with_cohort<R>(
    scope: BenchmarkCoverage,
    projects: usize,
    covered: &[&str],
    cohort: &CohortSelection,
    operate: impl FnOnce(&CorpusMeasurement<'_>) -> R,
) -> R {
    let certificate = certificate(covered);
    let invalid_syntax = ryo::linter::RuleId::try_from("invalid-syntax").expect("rule identifier");
    let suffixes = [SourceSuffix(".fixture")];
    operate(&CorpusMeasurement {
        bench: std::path::Path::new(BENCH),
        projects: (0..projects)
            .map(|index| Project {
                name: format!("project-{index}").into(),
                repository: "https://example.invalid/project.git".into(),
                revision: "main".into(),
                patch: "project.patch".into(),
            })
            .collect(),
        scope,
        profile: SourceProfile::Full,
        cache: std::path::Path::new("cache"),
        frozen_workload: std::path::Path::new("workload"),
        certificate: &certificate,
        mode: &MeasurementMode::Diagnostic,
        cohort,
        rule_context: &RuleContextProfile::Undeclared,
        search_roots: &SearchRoots::Undeclared,
        context_mode: ProjectContextMode::Isolated,
        language: CorpusLanguage {
            display: LanguageTitle("Fixture"),
            reference_display: ToolTitle("Reference"),
            reference_name: SampleLabel(
                ryo_benchmark_contracts::domain::receipts::constants::REFERENCE,
            ),
            suffixes: &suffixes,
            invalid_syntax: &invalid_syntax,
            valid_profile: ProfileLabel("accepted-fixture"),
            full_profile: ProfileLabel("all-fixture"),
        },
    })
}
