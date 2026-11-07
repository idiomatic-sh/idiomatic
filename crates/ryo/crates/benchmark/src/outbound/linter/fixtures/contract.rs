//! Every linter prepares the requested workspace and refuses operational failure.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::linter::interface::LinterPort;
use ryo_benchmark_contracts::domain::measurements::types::{Invocation, ProcessOutput};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use std::{path::PathBuf, time::Duration};
#[derive(Clone, Copy, Debug)]
/// Operations covered by the shared linter contract.
pub enum Given {
    /// Prepare the requested working directory.
    Prepare,
    /// Decode an operational process failure.
    OperationalFailure,
    /// Ask for the rule configuration support the adapter declares.
    RuleConfiguration,
    /// Spell the canonical texts of the identity inputs twice.
    CanonicalTexts,
    /// Spell the settings text of one invocation twice.
    SettingsText,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Canonical contract failure classifications.
pub enum Refusal {
    /// The process reported an operational failure.
    Operational,
    /// The implementation returned an unexpected failure category.
    Unexpected,
}
/// Named observations every implementation must satisfy.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("prepare uses requested working directory"),
        word: Word::Succeeds,
        input: || Input::Prepare,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("operational failure is not findings"),
        word: Word::Refuses,
        input: || Input::OperationalFailure,
        expect: Expect::Error(Failure::Operational),
    },
    Case {
        name: CaseName("rule configuration support is stable"),
        word: Word::Succeeds,
        input: || Input::RuleConfiguration,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("settings text is deterministic"),
        word: Word::Succeeds,
        input: || Input::SettingsText,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("canonical texts are deterministic"),
        word: Word::Succeeds,
        input: || Input::CanonicalTexts,
        expect: Expect::Value(true),
    },
];
/// Run the shared observations against the supplied implementation.
///
/// # Panics
///
/// Panics when fixture setup fails or the implementation violates a contract observation.
#[expect(
    clippy::expect_used,
    reason = "isolated fixture setup must fail the contract test"
)]
pub fn check(adapter: &impl LinterPort) {
    let temporary = tempfile::tempdir().expect("isolated linter fixture");
    let request = RunRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen,
        rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
        file_metadata: ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: PathBuf::new(),
        state: temporary.path().join("state"),
        files: vec![ryo::linter::RelativePath("input.txt".into())],
    };
    let subject = |input| {
        match input {
        Input::Prepare => adapter
            .prepare(&request)
            .map(|call| call.directory == request.root)
            .map_err(|_not_text| Failure::Unexpected),
        Input::RuleConfiguration => {
            let first = adapter.rule_configuration();
            let second = adapter.rule_configuration();
            Ok(first == second)
        }
        Input::SettingsText => {
            let case = super::data::case();
            let tool = super::data::tool();
            let invocation = Invocation {
                executable: "fixture".into(),
                arguments: Vec::new(),
                environment: std::collections::BTreeMap::default(),
                directory: ".".into(),
            };
            let first = adapter
                .settings_text(&invocation, &case, &tool)
                .map_err(|_not_text| Failure::Unexpected)?;
            let second = adapter
                .settings_text(&invocation, &case, &tool)
                .map_err(|_not_text| Failure::Unexpected)?;
            Ok(first == second)
        }
        Input::CanonicalTexts => {
            let context = ryo_benchmark_contracts::domain::context::types::PreparedRuleContext {
                tables: ryo_benchmark_contracts::domain::context::types::RuleContextTables {
                    selected: ryo_benchmark_contracts::domain::context::types::TableName("selected".into()),
                    disabled: ryo_benchmark_contracts::domain::context::types::TableName("disabled".into()),
                    external: ryo_benchmark_contracts::domain::context::types::TableName("external".into()),
                },
                configuration: ryo_benchmark_contracts::domain::context::types::RuleConfiguration {
                    external_prefixes: Vec::new(),
                    disabled_rules: Default::default(),
                },
            };
            Ok(adapter.rule_context_text(&context) == adapter.rule_context_text(&context))
        }
        Input::OperationalFailure => match adapter.decode(
            &request,
            ProcessOutput {
                signal: ryo_benchmark_contracts::domain::measurements::types::TerminationSignal::Exited,
                status: ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode(2),
                stdout: ryo_benchmark_contracts::domain::measurements::types::CapturedBytes(Vec::new()),
                stderr: ryo_benchmark_contracts::domain::measurements::types::CapturedBytes(Vec::new()),
                wall: Duration::ZERO,
                resources: ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::WallOnly,
            },
        ) {
            Err(Error::Invalid(_)) => Err(Failure::Operational),
            Err(_) => Err(Failure::Unexpected),
            Ok(_) => Ok(false),
        },
    }
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}

/// Classified output normalization refusal.
type Failure = Refusal;

/// Independently specified process output and request.
type Input = Given;

/// Whether normalization preserves the expected report.
type Value = bool;
