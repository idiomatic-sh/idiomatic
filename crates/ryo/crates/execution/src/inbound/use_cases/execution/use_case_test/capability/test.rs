//! Recursive producers and roots use the actual injected semantic capability, with terminal refusal.
use super::super::super::fixtures;
use super::super::super::fixtures::requests::{Authored, Preparation};
use crate::inbound::use_cases::execution::interface::CaptureIntent;
use crate::inbound::use_cases::execution::interface::DiagnosticDelivery;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase;
use crate::inbound::use_cases::execution::interface::FileRuleIntent;
use crate::inbound::use_cases::execution::interface::ObservationDemand;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::ProgramFileSessionPort as _;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort as _;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
        plan_parts::types::{ColumnIndex, HostedNumber, Input as Source, RepeatedRows, StepId},
        plans::types::Operation,
        query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
        rule_requests::types::{RuleParameters, RuleSpec},
        runs::types::{RuleApplicability, RuleId},
        stored_facts::types::Digest,
    },
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
#[derive(Clone, Copy, Debug)]
enum Refusal {
    None,
    Union,
    Project,
    Append,
}
#[derive(Debug)]
struct ObservedSemantics {
    refusal: Refusal,
    unions: AtomicUsize,
    projects: AtomicUsize,
    appends: AtomicUsize,
}
impl ryo_execution_runtime::outbound::operators::interface::OperatorsPort for ObservedSemantics {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "the port counts and refuses unions, projections and appends; every other operator call goes to the owner's operators"
        )]
        let refuse = match &call {
            OperatorCall::Evaluate {
                operation:
                    Operation::Union { .. }
                    | Operation::Intersection { .. }
                    | Operation::Difference { .. }
                    | Operation::Antijoin { .. },
                ..
            } => {
                self.unions.fetch_add(1, Ordering::Relaxed);
                matches!(self.refusal, Refusal::Union)
            }
            OperatorCall::Evaluate {
                operation: Operation::Project { .. },
                ..
            } => {
                self.projects.fetch_add(1, Ordering::Relaxed);
                matches!(self.refusal, Refusal::Project)
            }
            OperatorCall::Evaluate {
                operation:
                    Operation::Extend { .. }
                    | Operation::SpanBounds { .. }
                    | Operation::SpanBetween { .. }
                    | Operation::Replace { .. },
                ..
            } => {
                self.appends.fetch_add(1, Ordering::Relaxed);
                matches!(self.refusal, Refusal::Append)
            }
            _ => false,
        };
        if refuse {
            Err(PortableOperatorFault::ShapeMismatch)
        } else {
            fixtures::operators().operators(call)
        }
    }
}
/// What a rule's root emitted on one demand.
#[derive(Debug, PartialEq, Eq)]
enum Emitted {
    Complete(Cow<'static, [Cell]>),
    Failed(OperatingError),
    /// Any other root outcome: partial, skipped, unknown, no query or deferred.
    Other,
}
/// What the two demands emitted and how often the port saw each operator family.
#[derive(Debug, PartialEq, Eq)]
struct Observed {
    emitted: [Emitted; 2],
    /// Unions (and set operations), projections and appends the port evaluated.
    calls: (usize, usize, usize),
}
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use std::borrow::Cow;
type Input = Refusal;
type Value = Observed;
type Failure = OperatingError;
fn subject(refusal: Input) -> Result<Observed, Failure> {
    let port = Arc::new(ObservedSemantics {
        refusal,
        unions: AtomicUsize::new(0),
        projects: AtomicUsize::new(0),
        appends: AtomicUsize::new(0),
    });
    let engine = Preparation::new(port.clone(), Arc::new(fixtures::derived()));
    let mut plan = inputs::plan(vec![
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        Operation::Hosted {
            relation: HostedNumber(1),
        },
        Operation::Union {
            left: Source::Step(StepId(0)),
            right: Source::Step(StepId(1)),
        },
        Operation::Project {
            from: Source::Step(StepId(2)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
    ]);
    if matches!(refusal, Refusal::Append) {
        plan.steps[3].operation = Operation::Extend {
            from: Source::Step(StepId(2)),
            values: vec![ryo_values::domain::plan_parts::types::Operand::Literal(
                ryo_values::domain::plan_parts::types::QueryLiteral::Number(
                    ryo_relations::domain::relations::types::Integer(7),
                ),
            )],
        };
    }
    // An authored identity boundary keeps the union a genuine recursive producer.
    plan.frontier.push(Source::Step(StepId(2)));
    let prepared = engine.prepare(Authored {
        plans: vec![plan],
        bindings: vec![vec![]],
        rules: vec![RuleSpec {
            id: RuleId("injected-semantics".into()),
            query: QueryBundle {
                root: QueryDocumentSource {
                    id: QuerySourceId("injected-semantics".into()),
                    text: "".into(),
                },
                modules: Default::default(),
            },
            applicability: RuleApplicability::Always,
            given: vec![],
            observations: vec![],
            spelled: RuleParameters("".into()),
            include: vec![],
            exclude: vec![],
        }],
        digests: vec![Digest([0; 16])],
        hosted: vec![
            fixtures::requests::numbers("left", 1, &[&[1], &[1], &[2]]),
            fixtures::requests::numbers("right", 1, &[&[2], &[3]]),
        ],
        observations: ObservationIdentity::Independent,
    })?;
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("injected.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let request = fixtures::requests::execution().execution(&prepared, &corpus, None);
    let elements = Elements::default();
    let mut symbols = Symbols::default();
    let no_provider = |_| Err(OperatingError::NotRead);
    let mut file = request.open_file(
        FileInput {
            language: SourceLanguage::Recognized,
            resident_stability: ResidentStability::Stable,
            file: ryo_relations::domain::relations::types::FileId(0),
            evaluation: &[],
            publication: &[],
            intents: &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            }],
            source: ryo_values::domain::files::types::SourceView(b""),
            elements: &elements,
            read: &no_provider,
            semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
            kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
        },
        &mut symbols,
    )?;
    let demand = crate::inbound::use_cases::execution::fixtures::requests::emitted();
    let mut emitted = [Emitted::Other, Emitted::Other];
    for slot in &mut emitted {
        let outcome = file.rule(RuleIndex(0), demand, &mut symbols)?;
        *slot = match outcome.emit {
            RootOutcome::Complete(table) => Emitted::Complete(table.cells.into()),
            RootOutcome::Failed(error) => Emitted::Failed(error),
            RootOutcome::Partial { .. }
            | RootOutcome::Skipped(_)
            | RootOutcome::Unknown(_)
            | RootOutcome::NoQuery
            | RootOutcome::Deferred { .. } => Emitted::Other,
        };
    }
    let calls = (
        port.unions.load(Ordering::Relaxed),
        port.projects.load(Ordering::Relaxed),
        port.appends.load(Ordering::Relaxed),
    );
    file.finish()?;
    request.finish();
    Ok(Observed { emitted, calls })
}
const NUMBERS: Emitted = Emitted::Complete(Cow::Borrowed(&[
    Cell::Number(Integer(1)),
    Cell::Number(Integer(2)),
    Cell::Number(Integer(3)),
]));
const REFUSED: Emitted = Emitted::Failed(OperatingError::ShapeMismatch);
const fn observed(
    emitted: [Emitted; 2],
    calls: (usize, usize, usize),
) -> Expect<Observed, Failure> {
    Expect::Value(Observed { emitted, calls })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("append_refusal_preserves_the_injected_typed_fault_and_terminal_retry"),
        word: Word::Succeeds,
        input: || Refusal::Append,
        expect: observed([REFUSED, REFUSED], (5, 0, 1)),
    },
    Case {
        name: CaseName("recursive_producer_and_root_use_injected_semantics_once"),
        word: Word::Succeeds,
        input: || Refusal::None,
        expect: observed([NUMBERS, NUMBERS], (5, 3, 0)),
    },
    Case {
        name: CaseName("recursive_semantic_refusal_is_terminal_and_never_publishes"),
        word: Word::Succeeds,
        input: || Refusal::Union,
        expect: observed([REFUSED, REFUSED], (1, 0, 0)),
    },
    Case {
        name: CaseName("root_semantic_refusal_does_not_repeat_completed_producer"),
        word: Word::Succeeds,
        input: || Refusal::Project,
        expect: observed([REFUSED, REFUSED], (5, 1, 0)),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
