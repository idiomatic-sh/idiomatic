//! The shared preparation boundary rejects physical drift before linter preparation.
use super::service::{candidate, run as run_case};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::{execution::fixtures::fake::ExecutionFake, linter::interface::LinterPort};
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::measurements::types::{Invocation, ProcessOutput};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{
    CoverageAccounting, ExecutionMetadata, ProjectContext, RunReport,
};
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration,
    RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
};
use ryo_benchmark_contracts::domain::workloads::types::{
    CaseComparison, PreparedCase, PreparedReferenceBehavior,
};
use std::{cell::Cell, collections::BTreeMap};

/// Which physical drift a row exercises.
#[derive(Clone, Copy)]
enum Scenario {
    #[cfg(unix)]
    ModeDrift,
    DeclaredMissing,
}

/// What a row is given.
type Input = Scenario;
/// What the scenario observed at each check, in order, joined by `|`.
type Value = &'static str;
/// A scenario is never refused.
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    #[cfg(unix)]
    Case {
        name: CaseName(
            "metadata drift blocks each attempt before prepare with a live positive control",
        ),
        word: Word::Succeeds,
        input: || Scenario::ModeDrift,
        expect: Expect::Value("true|1|true|0|true|1|true|true|1|true|0|true|1|true"),
    },
    Case {
        name: CaseName("a declared missing primary is reverified before both contestants prepare"),
        word: Word::Succeeds,
        input: || Scenario::DeclaredMissing,
        expect: Expect::Value("true|true|2|false|false|0|true|true|2|true"),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let observed = match scenario {
        #[cfg(unix)]
        Scenario::ModeDrift => metadata_drift_blocks_each_attempt(),
        Scenario::DeclaredMissing => declared_missing_primary_is_reverified(),
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

struct ObservedPrepare(Cell<usize>);
impl LinterPort for ObservedPrepare {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Unsupported
    }
    fn prepare(&self, _: &RunRequest) -> Result<Invocation, Error> {
        self.0.set(self.0.get() + 1);
        Err(Error::Invalid("preparation reached".into()))
    }
    fn decode(&self, _: &RunRequest, _: ProcessOutput) -> Result<RunReport, Error> {
        unreachable!("this admission test stops at prepare")
    }
    fn settings_text(
        &self,
        normalized: &Invocation,
        _case: &ryo_benchmark_contracts::domain::workloads::types::PreparedCase,
        _profile: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    ) -> Result<crate::outbound::linter::types::CanonicalText, Error> {
        Ok(crate::outbound::linter::types::CanonicalText(format!(
            "{normalized:?}"
        )))
    }
    fn rule_context_text(
        &self,
        prepared: &ryo_benchmark_contracts::domain::context::types::PreparedRuleContext,
    ) -> crate::outbound::linter::types::CanonicalText {
        crate::outbound::linter::types::CanonicalText(format!("{prepared:?}"))
    }
}
#[cfg(unix)]
fn set_mode(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}
#[cfg(not(unix))]
fn set_mode(_path: &std::path::Path, _mode: u32) {}
#[cfg(unix)]
fn metadata_drift_blocks_each_attempt() -> String {
    let mut seen: Vec<String> = Vec::new();
    for expected in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("input.txt");
        std::fs::write(&path, b"same bytes").unwrap();
        let mode = if expected { 0o744 } else { 0o644 };
        set_mode(&path, mode);
        let file = ryo::linter::RelativePath("input.txt".into());
        let case = PreparedCase {
            comparison: CaseComparison::Full,
            rule: ryo::linter::RuleId::try_from("A001").unwrap(),
            name: "mode".into(),
            request: RunRequest {
                performance: Default::default(),
                selected_rules: CohortSelection::Frozen,
                rule_observations: InputDeclaration::Undeclared,
                rule_context: RuleContextPreparation::Unconfigured,
                project_search_roots: SearchRoots::Undeclared,
                given_project_inventory: Default::default(),
                staged_sources: StagedSources::Unlisted,
                file_metadata: FileMetadataDeclaration::Declared(BTreeMap::from([(
                    file.clone(),
                    if expected {
                        FileExecutable::Set
                    } else {
                        FileExecutable::Unset
                    },
                )])),
                context_manifest: ContextSeal::Unsealed,
                root: root.path().to_path_buf(),
                files: vec![file.clone()],
                state: root.path().join("cache"),
            },
            expected: RunReport {
                execution_metadata: ExecutionMetadata::Complete,
                project_context: ProjectContext::NotRequested,
                diagnostics: Vec::new(),
                skipped: Vec::new(),
                coverage: CoverageAccounting::Unaccounted,
                files: vec![file],
            },
            reference_discrepancy: PreparedReferenceBehavior::Agrees,
        };
        let execution = ExecutionFake::default();
        let reference = ObservedPrepare(Cell::new(0));
        let result = run_case(
            &crate::outbound::suite::factory::local(),
            &crate::outbound::context::factory::local(),
            &reference,
            &execution,
            &case,
            root.path(),
        );
        seen.push(format!(
            "{:?}",
            result
                .result
                .unwrap_err()
                .to_string()
                .contains("preparation reached")
        ));
        seen.push(format!("{:?}", reference.0.get()));
        set_mode(&path, if expected { 0o644 } else { 0o744 });
        let observed = ObservedPrepare(Cell::new(0));
        let blocked = candidate(
            &crate::outbound::suite::factory::local(),
            &crate::outbound::context::factory::local(),
            &observed,
            &execution,
            &case.request,
        )
        .unwrap_err();
        seen.push(format!(
            "{:?}",
            !blocked.to_string().contains("preparation reached")
        ));
        seen.push(format!("{:?}", observed.0.get()));
        set_mode(&path, mode);
        seen.push(format!(
            "{:?}",
            candidate(
                &crate::outbound::suite::factory::local(),
                &crate::outbound::context::factory::local(),
                &observed,
                &execution,
                &case.request
            )
            .unwrap_err()
            .to_string()
            .contains("preparation reached")
        ));
        seen.push(format!("{:?}", observed.0.get()));
        seen.push(format!(
            "{:?}",
            execution.history.lock().unwrap().is_empty()
        ));
    }
    seen.join("|")
}

fn declared_missing_primary_is_reverified() -> String {
    let mut seen: Vec<String> = Vec::new();
    use crate::outbound::context::interface::ContextPort as _;
    use ryo_benchmark_contracts::domain::context::types::CaseContext;
    let scratch = tempfile::tempdir().unwrap();
    let boundary = scratch.path().join("snapshot");
    let sources = [(
        ryo::linter::RelativePath("companion.py".into()),
        ryo::linter::SourceText("x = 1\n".into()),
    )]
    .into();
    let manifest = crate::outbound::context::factory::local()
        .materialize(
            &boundary,
            &CaseContext {
                logical_root: "project".into(),
                auxiliary_files: Default::default(),
                missing: vec!["project/gone.py".into()],
            },
            &sources,
        )
        .unwrap();
    let root = boundary.join("project");
    let files = vec![
        ryo::linter::RelativePath("companion.py".into()),
        ryo::linter::RelativePath("gone.py".into()),
    ];
    let case = PreparedCase {
        comparison: CaseComparison::Full,
        rule: ryo::linter::RuleId::try_from("R1").unwrap(),
        name: "missing".into(),
        request: RunRequest {
            performance: Default::default(),
            selected_rules: CohortSelection::Frozen,
            rule_observations: InputDeclaration::Undeclared,
            rule_context: RuleContextPreparation::Unconfigured,
            project_search_roots: SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            staged_sources: StagedSources::Unlisted,
            file_metadata: FileMetadataDeclaration::Undeclared,
            context_manifest: ContextSeal::Sealed(manifest),
            root: root.clone(),
            files: files.clone(),
            state: scratch.path().join("cache"),
        },
        expected: RunReport {
            execution_metadata: ExecutionMetadata::Complete,
            project_context: ProjectContext::NotRequested,
            diagnostics: Vec::new(),
            skipped: Vec::new(),
            coverage: CoverageAccounting::Unaccounted,
            files,
        },
        reference_discrepancy: PreparedReferenceBehavior::Agrees,
    };
    let execution = ExecutionFake::default();
    let observed = ObservedPrepare(Cell::new(0));
    for appeared in [false, true, false] {
        let path = root.join("gone.py");
        if appeared {
            std::fs::write(&path, "now present\n").unwrap();
        } else if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }
        let before = observed.0.get();
        let candidate = candidate(
            &crate::outbound::suite::factory::local(),
            &crate::outbound::context::factory::local(),
            &observed,
            &execution,
            &case.request,
        );
        let reference = run_case(
            &crate::outbound::suite::factory::local(),
            &crate::outbound::context::factory::local(),
            &observed,
            &execution,
            &case,
            scratch.path(),
        )
        .result;
        for result in [candidate, reference] {
            let message = result.unwrap_err().to_string();
            seen.push(format!("{:?}", message.contains("preparation reached")));
        }
        seen.push(format!("{:?}", observed.0.get() - before));
    }
    seen.push(format!(
        "{:?}",
        execution.history.lock().unwrap().is_empty()
    ));
    seen.join("|")
}
