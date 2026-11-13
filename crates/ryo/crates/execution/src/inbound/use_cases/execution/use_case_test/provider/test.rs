//! Binary operators must demand the refusing RHS after a real nonempty successful LHS.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::CaptureIntent;
use crate::inbound::use_cases::execution::interface::DiagnosticDelivery;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase;
use crate::inbound::use_cases::execution::interface::FileRuleIntent;
use crate::inbound::use_cases::execution::interface::ObservationDemand;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::ProgramFileSessionPort as _;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_relations::domain::relations::{
    service::{kinds, node_id},
    types::{FileId, Offset, Position, Relation, Span, Value as Cell},
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Input as PlanInput, StepId},
        plans::types::Operation,
        rows::types::{RowCount, Table},
    },
};
use std::cell::RefCell;
#[derive(Clone, Copy)]
enum Binary {
    Union,
    Intersection,
    Difference,
    Antijoin,
}
#[derive(Clone, Copy)]
enum Missing {
    First,
    Second,
    Neither,
}
#[derive(Clone, Copy)]
enum Delivery {
    Collected,
    Forwarded,
}
use std::borrow::Cow;
/// The relations the providers were asked for, in order.
type History = Cow<'static, [Relation]>;
/// A forwarded delivery: the rows its first emission and its retry published, the retry's answer, and whether the retry
/// read any provider again.
#[derive(Debug, PartialEq, Eq)]
struct Forwarded {
    callbacks: usize,
    retry: Option<OperatingError>,
    reread: bool,
}
/// The provider reads, and the forwarded delivery when the rule streamed.
#[derive(Debug, PartialEq, Eq)]
struct Delivered {
    history: History,
    forwarded: Option<Forwarded>,
}
/// A refused rule: the error and what was delivered before and after it.
#[derive(Debug, PartialEq, Eq)]
struct Refused {
    error: OperatingError,
    delivered: Delivered,
}
type Input = (Binary, Missing, Delivery);
type Value = Delivered;
type Failure = Refused;
fn subject((operator, missing, delivery): Input) -> Result<Value, Failure> {
    let (elements, mut symbols) = fixtures::requests::tree_with_names();
    // Actual Function declaration and valid typed cells; no numeric catalog stand-in.
    let function = Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(1),
        cells: vec![
            Cell::Node(node_id(FileId(0), Position(1))),
            Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
            Cell::File(FileId(0)),
            Cell::Span(Span {
                start: Offset(0),
                end: Offset(1),
            }),
        ],
    };
    let comment = Table {
        kinds: kinds(Relation::Comment).to_vec(),
        count: RowCount(0),
        cells: vec![],
    };
    let reads = RefCell::new(Vec::new());
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "the fake serves the two relations the rule reads; every other relation is not read"
    )]
    let read = |relation| {
        reads.borrow_mut().push(relation);
        match relation {
            Relation::Function if matches!(missing, Missing::Second | Missing::Neither) => {
                Ok(SourceRows::Computed(&function))
            }
            Relation::Comment if matches!(missing, Missing::Neither) => {
                Ok(SourceRows::Computed(&comment))
            }
            _ => Err(OperatingError::NotRead),
        }
    };
    let left = PlanInput::Step(StepId(0));
    let right = PlanInput::Step(StepId(1));
    let operation = match operator {
        Binary::Union => Operation::Union { left, right },
        Binary::Intersection => Operation::Intersection { left, right },
        Binary::Difference => Operation::Difference { left, right },
        Binary::Antijoin => Operation::Antijoin {
            left,
            right,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        },
    };
    let plan = inputs::plan(vec![
        Operation::Select {
            from: PlanInput::Relation(Relation::Function),
            comparisons: vec![],
            kept: vec![ColumnIndex(2)],
        },
        Operation::Select {
            from: PlanInput::Relation(Relation::Comment),
            comparisons: vec![],
            kept: vec![ColumnIndex(2)],
        },
        operation,
    ]);
    let (result, forwarded) = match delivery {
        Delivery::Collected => (
            fixtures::requests::published_from(
                plan,
                vec![],
                vec![],
                b"abcd",
                &elements,
                &mut symbols,
                &read,
            )
            .map(|_| ()),
            None,
        ),
        Delivery::Forwarded => {
            let stream = || -> Result<(Result<(), OperatingError>, Forwarded), OperatingError> {
                use ryo_values::domain::files::types::{
                    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
                };
                let (engine, prepared) = fixtures::requests::prepared(plan, vec![], vec![])?;
                let corpus = [CorpusInput::Available(CorpusEntry {
                    path: ryo_language::domain::files::types::FilePath(
                        "provider-refusal.py".into(),
                    ),
                    size: FileSize(4),
                    modified: Nanoseconds(0),
                    executable: ExecutablePermission::Unknown,
                })];
                let request = fixtures::requests::execution().execution(&prepared, &corpus, None);
                let mut file = request.open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &[FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Forwarded,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::Disabled,
                    }],
                    source: ryo_values::domain::files::types::SourceView(b"abcd"),
                    elements: &elements,
                    read: &read,
                    semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
                },
                &mut symbols,
            )?;
                let mut callbacks = 0;
                let result = file
                    .emit_rows(RuleIndex(0), &mut symbols, &mut |_| {
                        callbacks += 1;
                        Ok(())
                    })
                    .map(|_| ());
                let terminal_reads = reads.borrow().clone();
                let retry = file
                    .emit_rows(RuleIndex(0), &mut symbols, &mut |_| {
                        callbacks += 1;
                        Ok(())
                    })
                    .err();
                let forwarded = Forwarded {
                    callbacks,
                    retry,
                    reread: *reads.borrow() != terminal_reads,
                };
                file.finish()?;
                request.finish();
                Ok((result, forwarded))
            };
            stream().map_or_else(
                |error| (Err(error), None),
                |(result, forwarded)| (result, Some(forwarded)),
            )
        }
    };
    let delivered = Delivered {
        history: reads.into_inner().into(),
        forwarded,
    };
    match result.err() {
        Some(error) => Err(Refused { error, delivered }),
        None => Ok(delivered),
    }
}
/// The reads of a rule whose left provider answers: the left, the right, then the left's pinned borrows again.
const BOTH_READ: &[Relation] = &[
    Relation::Function,
    Relation::Comment,
    Relation::Function,
    Relation::Function,
    Relation::Function,
];
const fn refused(
    history: &'static [Relation],
    forwarded: Option<Forwarded>,
) -> Expect<Value, Failure> {
    Expect::Error(Refused {
        error: OperatingError::NotRead,
        delivered: Delivered {
            history: Cow::Borrowed(history),
            forwarded,
        },
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("union_over_two_available_providers_publishes"),
        word: Word::Succeeds,
        input: || (Binary::Union, Missing::Neither, Delivery::Collected),
        expect: Expect::Value(Delivered {
            history: Cow::Borrowed(BOTH_READ),
            forwarded: None,
        }),
    },
    Case {
        name: CaseName(
            "forwarded_union_requires_rhs_after_successful_lhs_and_never_delivers_a_prefix",
        ),
        word: Word::Refuses,
        input: || (Binary::Union, Missing::Second, Delivery::Forwarded),
        expect: refused(
            BOTH_READ,
            Some(Forwarded {
                callbacks: 0,
                retry: Some(OperatingError::NotRead),
                reread: false,
            }),
        ),
    },
    Case {
        name: CaseName("union_requires_rhs_after_successful_lhs"),
        word: Word::Refuses,
        input: || (Binary::Union, Missing::Second, Delivery::Collected),
        expect: refused(BOTH_READ, None),
    },
    Case {
        name: CaseName("intersection_requires_rhs_after_successful_lhs"),
        word: Word::Refuses,
        input: || (Binary::Intersection, Missing::Second, Delivery::Collected),
        expect: refused(BOTH_READ, None),
    },
    Case {
        name: CaseName("difference_requires_rhs_after_successful_lhs"),
        word: Word::Refuses,
        input: || (Binary::Difference, Missing::Second, Delivery::Collected),
        expect: refused(BOTH_READ, None),
    },
    Case {
        name: CaseName("antijoin_requires_rhs_after_successful_lhs"),
        word: Word::Refuses,
        input: || (Binary::Antijoin, Missing::Second, Delivery::Collected),
        expect: refused(BOTH_READ, None),
    },
    Case {
        name: CaseName("antijoin_first_provider_failure_prevents_rhs_demand"),
        word: Word::Refuses,
        input: || (Binary::Antijoin, Missing::First, Delivery::Collected),
        expect: refused(&[Relation::Function], None),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
