//! Recursive producers and roots use the actual injected semantic capability, with terminal refusal.
use crate::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use crate::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use crate::inbound::use_cases::execution::use_case::PrepareUseCaseImpl;
use crate::inbound::use_cases::prepare::interface::PrepareUseCase;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase;
use super::super::super::{fixtures, use_case::ExecutionUseCaseImpl};
use crate::application::file_dag::program::file_session::types::FileInput;
use crate::application::file_dag::program::file_session::types::SourceLanguage;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::inbound::use_cases::execution::interface::CaptureIntent;
use crate::inbound::use_cases::execution::interface::DiagnosticDelivery;
use crate::inbound::use_cases::execution::interface::EmitDemand;
use crate::inbound::use_cases::execution::interface::FileRuleIntent;
use crate::inbound::use_cases::execution::interface::ObservationDemand;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::RuleDemand;
use crate::inbound::use_cases::execution::interface::RuleIndex;
use crate::inbound::use_cases::prepare::interface::ObservationIdentity;
use crate::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
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
impl OperatorsUseCase for ObservedSemantics {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        let refuse = match &call {
            OperatorCall::SetRow { .. } => {
                self.unions.fetch_add(1, Ordering::Relaxed);
                matches!(self.refusal, Refusal::Union)
            }
            OperatorCall::Project { .. } => {
                self.projects.fetch_add(1, Ordering::Relaxed);
                matches!(self.refusal, Refusal::Project)
            }
            OperatorCall::Append { .. } => {
                self.appends.fetch_add(1, Ordering::Relaxed);
                matches!(self.refusal, Refusal::Append)
            }
            _ => false,
        };
        if refuse {
            Err(PortableOperatorFault::ShapeMismatch)
        } else {
            OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())).operators(call)
        }
    }
}
type Input = Refusal;
type Value = ();
type Failure = OperatingError;
fn subject(refusal: Input) -> Result<Value, Failure> {
    let port = Arc::new(ObservedSemantics {
        refusal,
        unions: AtomicUsize::new(0),
        projects: AtomicUsize::new(0),
        appends: AtomicUsize::new(0),
    });
    let engine = PrepareUseCaseImpl::new(
        port.clone(),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    );
    let mut plan = fixtures::plan(vec![
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
    let prepared = engine.prepare(PrepareRequest { plans: PlanSource::Authored { plans: vec![plan], bindings: vec![vec![]] }, rules: vec![RuleSpec {
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
        }], digests: vec![Digest([0; 16])], hosted: vec![
            fixtures::numbers("left", 1, &[&[1], &[1], &[2]]),
            fixtures::numbers("right", 1, &[&[2], &[3]]),
        ], observations: ObservationIdentity::Independent })?;
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("injected.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
    let elements = Elements::default();
    let mut symbols = Symbols::default();
    let no_provider = |_| Err(OperatingError::NotRead);
    let mut file = request.open_file(
        FileInput {
            language: SourceLanguage::Recognized,
            resident_stability: facts::ResidentStability::Stable,
            file: ryo_relations::domain::relations::types::FileId(0),
            evaluation: &[],
            publication: &[],
            intents: &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            }],
            source: b"",
            elements: &elements,
            read: &no_provider,
            semantic: None,
            kinds: None,
        },
        &mut symbols,
    )?;
    let demand = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    for _ in 0..2 {
        let outcome = file.rule(RuleIndex(0), demand, &mut symbols)?;
        match refusal {
            Refusal::None => {
                let RootOutcome::Complete(table) = outcome.emit else {
                    panic!("actual injected semantics must complete")
                };
                use ryo_relations::domain::relations::types::{Integer, Value};
                assert_eq!(table.count.0, 3);
                assert_eq!(table.cells, [1, 2, 3].map(|n| Value::Number(Integer(n))));
            }
            _ => match outcome.emit {
                RootOutcome::Failed(error) => {
                    assert_eq!(error, OperatingError::ShapeMismatch, "{refusal:?}")
                }
                _ => panic!("injected {refusal:?} refusal must remain an exact terminal failure"),
            },
        }
    }
    let expected = match refusal {
        Refusal::None => (5, 3, 0),
        Refusal::Union => (1, 0, 0),
        Refusal::Project => (5, 1, 0),
        Refusal::Append => (5, 0, 1),
    };
    assert_eq!(
        (
            port.unions.load(Ordering::Relaxed),
            port.projects.load(Ordering::Relaxed),
            port.appends.load(Ordering::Relaxed)
        ),
        expected,
        "replay never reruns the child or root, including refusal"
    );
    file.finish()?;
    request.finish();
    Ok(())
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("append_refusal_preserves_the_injected_typed_fault_and_terminal_retry"),
        word: Word::Succeeds,
        input: || Refusal::Append,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("recursive_producer_and_root_use_injected_semantics_once"),
        word: Word::Succeeds,
        input: || Refusal::None,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("recursive_semantic_refusal_is_terminal_and_never_publishes"),
        word: Word::Succeeds,
        input: || Refusal::Union,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("root_semantic_refusal_does_not_repeat_completed_producer"),
        word: Word::Succeeds,
        input: || Refusal::Project,
        expect: Expect::Value(()),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
