//! A reference that changes a declared file mode stops the second candidate before it prepares.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::verify_cases::fixtures::shard::{
    Refusal, Shape, category, history_len, prepared, set_mode, staged_case, succeeding,
    verify_shard,
};
use crate::outbound::linter::{fixtures::fake::LinterFake, interface::LinterPort};
use crate::outbound::receipts::fixtures::fake::ReceiptsFake;
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::measurements::types::{Invocation, ProcessOutput};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::{FileMetadataDeclaration, RunRequest};
use ryo_benchmark_contracts::domain::workloads::types::FileObservation;
use std::path::Path;

/// The executable bit the case declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Declared {
    Executable,
    Plain,
}

/// Whether the reference changes the mode on its second preparation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Drift {
    Changes,
    Keeps,
}

/// One declared mode and what the reference does to it.
#[derive(Clone, Copy)]
struct Scenario {
    declared: Declared,
    drift: Drift,
}

type Input = Scenario;
type Value = usize;
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    #[cfg(unix)]
    Case {
        name: CaseName("an unchanged non-executable mode lets both cases execute"),
        word: Word::Succeeds,
        input: || Scenario {
            declared: Declared::Plain,
            drift: Drift::Keeps,
        },
        expect: Expect::Value(2),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("an unchanged executable mode lets both cases execute"),
        word: Word::Succeeds,
        input: || Scenario {
            declared: Declared::Executable,
            drift: Drift::Keeps,
        },
        expect: Expect::Value(2),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("a reference that makes a file executable stops the second candidate"),
        word: Word::Refuses,
        input: || Scenario {
            declared: Declared::Plain,
            drift: Drift::Changes,
        },
        expect: Expect::Error(Refusal::MetadataDrift),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("a reference that clears the executable bit stops the second candidate"),
        word: Word::Refuses,
        input: || Scenario {
            declared: Declared::Executable,
            drift: Drift::Changes,
        },
        expect: Expect::Error(Refusal::MetadataDrift),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    mode_drift(scenario).map_err(|error| category(&error.to_string()))
}

#[cfg(unix)]
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(unix)]
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// A reference that, on its second preparation, changes the mode of the case file.
struct DriftingReference {
    calls: std::cell::Cell<usize>,
    declared: Declared,
    drift: Drift,
}
impl LinterPort for DriftingReference {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Unsupported
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        let calls = self.calls.get() + 1;
        self.calls.set(calls);
        if calls == 2 && self.drift == Drift::Changes {
            set_mode(
                &request.root.join("case.py"),
                if self.declared == Declared::Executable {
                    0o644
                } else {
                    0o744
                },
            );
        }
        LinterFake::default().prepare(request)
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        LinterFake::default().decode(request, output)
    }
    fn settings_text(
        &self,
        normalized: &Invocation,
        case: &ryo_benchmark_contracts::domain::workloads::types::PreparedCase,
        profile: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    ) -> Result<crate::outbound::linter::types::CanonicalText, Error> {
        LinterFake::default().settings_text(normalized, case, profile)
    }
    fn rule_context_text(
        &self,
        prepared: &ryo_benchmark_contracts::domain::context::types::PreparedRuleContext,
    ) -> crate::outbound::linter::types::CanonicalText {
        LinterFake::default().rule_context_text(prepared)
    }
}

fn mode_drift(scenario: Scenario) -> Result<Value, Error> {
    let Scenario { declared, drift } = scenario;
    let root = staged_case(if declared == Declared::Executable {
        0o744
    } else {
        0o644
    });
    let (mut suite, _) = prepared(Shape::Agreement);
    let profile = suite.profile.as_mut().unwrap();
    profile.defaults.truncate(1);
    profile.file_observation = FileObservation::PosixExecutable;
    let case = &mut suite.prepared[0];
    case.name = "first".into();
    case.expected.diagnostics.clear();
    case.request.root = root.path().to_path_buf();
    case.request.file_metadata =
        FileMetadataDeclaration::Declared(std::collections::BTreeMap::from([(
            case.request.files[0].clone(),
            if declared == Declared::Executable {
                FileExecutable::Set
            } else {
                FileExecutable::Unset
            },
        )]));
    let mut second = case.clone();
    second.name = "second".into();
    suite.prepared.push(second);
    let reference = DriftingReference {
        calls: std::cell::Cell::new(0),
        declared,
        drift,
    };
    let execution = succeeding();
    let result = verify_shard(
        &suite,
        &ReceiptsFake::default(),
        &reference,
        &LinterFake::default(),
        &execution,
        Path::new("rules"),
    );
    assert_eq!(reference.calls.get(), 2);
    if drift == Drift::Changes {
        assert_eq!(
            history_len(&execution),
            Some(3),
            "second candidate must not execute after reference changes mode"
        );
    } else {
        assert_eq!(history_len(&execution), Some(4));
    }
    result
}
