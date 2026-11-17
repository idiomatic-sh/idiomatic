//! One certified corpus measured over the fakes of every boundary it calls.
#![expect(
    clippy::expect_used,
    reason = "fixture rule identifiers and sealed values are test preconditions"
)]

use super::{
    types::{CorpusLanguage, CorpusMeasurement, ProfileLabel, SourceSuffix, ToolTitle},
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
    ExecutionProfile, Invocation, MeasurementMode, ResidentBytes, ResourceObservation, Resources,
};
use ryo_benchmark_contracts::domain::requests::types::{
    CertificationCases, CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration,
    RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
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
        probe: request,
        profiles: [profile.clone(), profile],
        execution: ExecutionProfile::Wall,
        cases: ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount(2),
    }
}

/// The use case wired to the fakes of every boundary it calls.
#[must_use]
pub fn use_case() -> MeasureCorpusUseCaseImpl {
    wired(
        &[],
        (
            Arc::new(WorkspaceFake),
            Arc::new(ExecutionFake::default()),
            LinterFake::default(),
        ),
        Arc::new(SuiteFake {
            profile: Some(crate::outbound::suite::fixtures::samples::tool()),
            ..SuiteFake::default()
        }),
        Arc::new(crate::outbound::host::fixtures::fake::HostFake::default()),
    )
}

/// The use case on a Microsoft kernel outside a container, whose suite's settings profile requires executable bits.
#[must_use]
pub fn unreliable_host_use_case() -> MeasureCorpusUseCaseImpl {
    wired(
        &[],
        (
            Arc::new(WorkspaceFake),
            Arc::new(ExecutionFake::default()),
            LinterFake::default(),
        ),
        Arc::new(SuiteFake {
            profile: Some(ryo_benchmark_contracts::domain::workloads::types::Tool {
                file_observation:
                    ryo_benchmark_contracts::domain::workloads::types::FileObservation::PosixExecutable,
                ..crate::outbound::suite::fixtures::samples::tool()
            }),
            ..SuiteFake::default()
        }),
        Arc::new(crate::outbound::host::fixtures::fake::HostFake {
            facts: ryo_benchmark_contracts::domain::hosts::types::HostFacts {
                family: ryo_benchmark_contracts::domain::hosts::types::HostFamily::Unix,
                kernel: ryo_benchmark_contracts::domain::hosts::types::HostKernel::Microsoft,
                isolation: ryo_benchmark_contracts::domain::hosts::types::HostIsolation::Bare,
            },
            ..crate::outbound::host::fixtures::fake::HostFake::default()
        }),
    )
}

/// The use case of `use_case`, its discovery answering every project with `files`, its workspace checking out `root`, tracking
/// exactly them, and the local suite reading their bytes.
#[must_use]
pub fn discovering(root: &std::path::Path, files: &[&str]) -> MeasureCorpusUseCaseImpl {
    over_project(
        root,
        files,
        (Arc::new(ExecutionFake::default()), LinterFake::default()),
    )
}

/// The use case of `discovering`, its every run exiting cleanly with one second of CPU and a peak of a megabyte collected, and
/// the candidate leaving a store of one ten-byte file: each run it measures has a cost.
#[must_use]
pub fn measuring(root: &std::path::Path, files: &[&str]) -> MeasureCorpusUseCaseImpl {
    over_project(
        root,
        files,
        (
            Arc::new(ExecutionFake {
                status: 0,
                resources: ResourceObservation::Collected(Resources {
                    cpu: std::time::Duration::from_secs(1),
                    peak_rss_bytes: ResidentBytes(1_048_576),
                }),
                ..ExecutionFake::default()
            }),
            LinterFake {
                store: crate::outbound::linter::fixtures::fake::KeptState::Store(vec![0; 10]),
                ..LinterFake::default()
            },
        ),
    )
}

/// The use case over the project `root` holding `files`, its runs executed by `execution` and its candidate `candidate`.
fn over_project(
    root: &std::path::Path,
    files: &[&str],
    (execution, candidate): (
        Arc<dyn crate::outbound::execution::interface::ExecutionPort>,
        LinterFake,
    ),
) -> MeasureCorpusUseCaseImpl {
    wired(
        files,
        (
            Arc::new(
                crate::outbound::workspace::fixtures::fake::ProjectWorkspace {
                    root: root.to_owned(),
                    tracked: files.iter().map(|file| (*file).to_owned()).collect(),
                },
            ),
            execution,
            candidate,
        ),
        Arc::new(crate::outbound::suite::factory::local()),
        Arc::new(crate::outbound::host::fixtures::fake::HostFake::default()),
    )
}

/// The use case over the fakes of every boundary, its discovery answering `files`, its workspace `workspace`, its execution
/// `execution`, its candidate `candidate`, its suite `suite` and its host `host`.
fn wired(
    files: &[&str],
    (workspace, execution, candidate): (
        Arc<dyn crate::outbound::workspace::interface::WorkspacePort>,
        Arc<dyn crate::outbound::execution::interface::ExecutionPort>,
        LinterFake,
    ),
    suite: Arc<dyn crate::outbound::suite::interface::SuitePort>,
    host: Arc<dyn crate::outbound::host::interface::HostPort>,
) -> MeasureCorpusUseCaseImpl {
    MeasureCorpusUseCaseImpl::new(
        Arc::new(LinterFake::default()),
        Arc::new(candidate),
        Arc::new(DiscoveryFake {
            files: files
                .iter()
                .map(|file| ryo::linter::RelativePath((*file).into()))
                .collect(),
        }),
        execution,
        suite,
        Arc::new(ContextFake),
        Arc::new(EvidenceFake),
        workspace,
        host,
        Arc::new(crate::outbound::engine::fixtures::fake::EngineFake::default()),
        Arc::new(crate::outbound::process_meter::fixtures::fake::ProcessMeterFake::default()),
    )
}

/// Hand `operate` a corpus of `projects`; each project is pinned to a branch name, which the workspace refuses.
///
/// # Panics
/// Never: the rule identifier is a valid literal.
pub fn with_corpus<R>(projects: usize, operate: impl FnOnce(&CorpusMeasurement<'_>) -> R) -> R {
    with_cohort(projects, &["A"], &CohortSelection::Frozen, operate)
}

/// Hand `operate` a corpus of `projects`, certified for the `covered` rules and measuring `cohort`.
///
/// # Panics
/// When a covered rule is not a valid rule identifier.
pub fn with_cohort<R>(
    projects: usize,
    covered: &[&str],
    cohort: &CohortSelection,
    operate: impl FnOnce(&CorpusMeasurement<'_>) -> R,
) -> R {
    measured(
        pinned(projects, "main"),
        (certificate(covered), MeasurementMode::Diagnostic),
        cohort,
        (
            std::path::Path::new(BENCH),
            std::path::Path::new("cache"),
            &[SourceSuffix(".fixture")],
        ),
        operate,
    )
}

/// Hand `operate` a corpus of one project pinned to a full commit, which the workspace checks out, matching the sources
/// that end in one of `suffixes`, its trials kept under `cache`.
pub fn with_suffixes<R>(
    (bench, cache): (&std::path::Path, &std::path::Path),
    suffixes: &[SourceSuffix<'_>],
    operate: impl FnOnce(&CorpusMeasurement<'_>) -> R,
) -> R {
    measured(
        pinned(1, &"0".repeat(40)),
        (certificate(&["A"]), MeasurementMode::Diagnostic),
        &CohortSelection::Frozen,
        (bench, cache, suffixes),
        operate,
    )
}

/// Hand `operate` a corpus of one project pinned to a full commit, measuring rule A as the complete inventory under a
/// certificate whose probe names the fixture contestants and the local suite holds, over the sources that end in one of `suffixes`, its trials
/// kept under `cache`.
///
/// # Panics
/// When the local suite cannot fingerprint the certificate's artifacts.
pub fn with_certificate<R>(
    (bench, cache): (&std::path::Path, &std::path::Path),
    suffixes: &[SourceSuffix<'_>],
    operate: impl FnOnce(&CorpusMeasurement<'_>) -> R,
) -> R {
    let mut sealed = certificate(&["A"]);
    sealed.fingerprint = crate::outbound::suite::interface::SuitePort::fingerprint(
        &crate::outbound::suite::factory::local(),
        crate::outbound::suite::types::ArtifactSet(&sealed.paths),
    )
    .expect("the certificate's artifacts fingerprinted");
    let rule = ryo::linter::RuleId::try_from("A").expect("rule identifier");
    measured(
        pinned(1, &"0".repeat(40)),
        (
            sealed,
            MeasurementMode::Benchmark {
                inventory: vec![rule.clone()],
            },
        ),
        &CohortSelection::Explicit(vec![rule]),
        (bench, cache, suffixes),
        operate,
    )
}

/// `count` projects, each pinned to `revision`.
fn pinned(count: usize, revision: &str) -> Vec<Project> {
    (0..count)
        .map(|index| Project {
            name: format!("project-{index}").into(),
            repository: "https://example.invalid/project.git".into(),
            revision: revision.into(),
            patch: "project.patch".into(),
        })
        .collect()
}

/// Hand `operate` the corpus of `projects`, certified for the `covered` rules, measuring `cohort` over the sources
/// that end in one of `suffixes`.
fn measured<R>(
    projects: Vec<Project>,
    (certificate, mode): (Certificate, MeasurementMode),
    cohort: &CohortSelection,
    (bench, cache, suffixes): (&std::path::Path, &std::path::Path, &[SourceSuffix<'_>]),
    operate: impl FnOnce(&CorpusMeasurement<'_>) -> R,
) -> R {
    let invalid_syntax = ryo::linter::RuleId::try_from("invalid-syntax").expect("rule identifier");
    let language = "fixture".into();

    operate(&CorpusMeasurement {
        bench,
        projects,
        growth: crate::domain::growth::types::CorpusGrowth::Ungrown,
        verification: super::types::VerifiedInventory {
            directory: std::path::Path::new(""),
            rules: &[],
        },
        cache,
        frozen_workload: std::path::Path::new("workload"),
        certificate: &certificate,
        mode: &mode,
        cohort,
        rule_context: &RuleContextProfile::Undeclared,
        search_roots: &SearchRoots::Undeclared,
        context_mode: ProjectContextMode::Isolated,
        language: CorpusLanguage {
            id: &language,
            display: LanguageTitle("Fixture"),
            reference_display: ToolTitle("Reference"),
            suffixes,
            invalid_syntax: &invalid_syntax,
            valid_profile: ProfileLabel("accepted-fixture"),
        },
    })
}

/// A bench directory whose settings profile, `benchmark.toml`, defaults to the `rules` and asks nothing of the host,
/// removed on drop: what the local suite reads when a run admits its rules.
///
/// # Panics
/// When the directory or its profile cannot be written.
#[must_use]
pub fn bench_of(rules: &[&str]) -> tempfile::TempDir {
    let bench = tempfile::tempdir().expect("a bench directory");
    let defaults = rules
        .iter()
        .map(|rule| format!("'{rule}'"))
        .collect::<Vec<_>>()
        .join(", ");
    std::fs::write(
        bench.path().join("benchmark.toml"),
        format!(
            "tool = 'fixture'\nversion = '1'\nsha256 = ['{}']\ndefaults = [{defaults}]\nparser-configuration = 'fixture-v1'\n",
            "0".repeat(64)
        ),
    )
    .expect("a settings profile");
    bench
}

/// A project directory holding `files`, each one line long, removed on drop.
///
/// # Panics
/// When the directory or a file cannot be written.
#[must_use]
pub fn project_of(files: &[&str]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a project directory");
    for file in files {
        std::fs::write(root.path().join(file), "a\n").expect("a source");
    }
    root
}
