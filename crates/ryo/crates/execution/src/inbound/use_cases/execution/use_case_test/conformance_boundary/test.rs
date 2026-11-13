//! Public canonical error/delivery parity and independent ordered-bag controls.
use super::super::conformance::test::{compare_delivery, same_rows, settled};
use crate::inbound::use_cases::execution::interface::OperatingError;
use OperationKind::{Join, Project, Select, SourceMatches, SpanBounds, Union};
use ryo_execution_domain::domain::coverage::types::{Lacking, Partial, PartialPlace};
use ryo_execution_domain::domain::operation_schema::{service::schema_of, types::InputShape};
use ryo_execution_domain::domain::trust::{
    service::{blocked, needs_of, scoped_needs},
    types::Blocking,
};
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::expectations::{
    Conformance, Fixture, SOURCE, conformance,
};
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::values::matrix;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::*;
use ryo_relations::domain::relations::types::{Integer, Relation, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::stored_facts::types::PartialCause;
use ryo_values::domain::{
    plan_parts::types::ColumnIndex, plans::types::Operation, query_operators::types::OperationKind,
};
use std::borrow::Cow;
/// What a row does with each kind's conformance fixture.
#[derive(Clone, Copy, Debug)]
enum Boundary {
    /// Delivers the fixture and checks its rows against the fixture's expected rows.
    Delivered,
    /// Joins `[1, 10], [1, 11], [2, 12]` with `[1, 20], [2, 21], [1, 22]` on column 0.
    KeyedJoin,
    /// Makes the fixture invalid: a column past its input, or a union's second input of another shape.
    Invalid,
    /// Computes the schema with the second input missing.
    MissingInput,
    /// Compares the delivered rows with the first dropped, with other rows, and with none.
    Altered,
    /// Plans the operation alone and reads each relation it reads in part.
    PartialInputs,
}

/// What one fixture showed.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Observed {
    /// The rows delivered, and how they compare with the fixture's expected rows.
    Delivered(usize, Conformance),
    /// The joined rows.
    Rows(Cow<'static, [[Cell; 2]]>),
    /// The schema's refusal, and those the collected and the streamed delivery answered.
    Refused([Option<OperatingError>; 3]),
    /// The schema's refusal.
    MissingInput(Option<OperatingError>),
    /// Whether the collected table is the delivered rows, the rows with the first dropped, the rows
    /// `[1], [2], [2]`, and no rows.
    Compared {
        delivered: bool,
        dropped: bool,
        other: bool,
        none: bool,
    },
    /// The plan's need sets, whether the first is empty, and for each relation read: whether a
    /// partial read of it blocks nothing, and whether the scoped needs read it.
    Needs {
        sets: usize,
        first_empty: bool,
        relations: Cow<'static, [(Relation, bool, bool)]>,
    },
}

/// The boundary, over each kind's fixture.
type Input = (Boundary, &'static [OperationKind]);
type Value = Cow<'static, [(OperationKind, Observed)]>;
type Failure = OperatingError;

/// The four unary fixtures.
const UNARY: &[OperationKind] = &[Select, Project, SourceMatches, SpanBounds];
/// The two binary fixtures.
const BINARY: &[OperationKind] = &[Join, Union];

/// A joined row of `left` and `right`.
const fn joined(left: i64, right: i64) -> [Cell; 2] {
    [Cell::Number(Integer(left)), Cell::Number(Integer(right))]
}

/// A column index past the input.
const COLUMN_99: OperatingError = OperatingError::Table(RowsError::NoSuchColumn { index: 99 });

/// The schema, the collected and the streamed delivery each refused with the column past the input.
const PAST_THE_INPUT: Observed =
    Observed::Refused([Some(COLUMN_99), Some(COLUMN_99), Some(COLUMN_99)]);
/// The schema, the collected and the streamed delivery each refused with unlike shapes.
const UNLIKE_SHAPES: Observed = Observed::Refused([
    Some(OperatingError::ShapeMismatch),
    Some(OperatingError::ShapeMismatch),
    Some(OperatingError::ShapeMismatch),
]);

/// A read of the function relation, free under a partial read and in the scoped needs.
const ONE_READ: &[(Relation, bool, bool)] = &[(Relation::Function, true, true)];
/// Reads of the function and scope relations, each free under a partial read and in the scoped needs.
const TWO_READS: &[(Relation, bool, bool)] = &[
    (Relation::Function, true, true),
    (Relation::Scope, true, true),
];

/// One empty need set over the relations read.
const fn needs_of_function(relations: &'static [(Relation, bool, bool)]) -> Observed {
    Observed::Needs {
        sets: 1,
        first_empty: true,
        relations: Cow::Borrowed(relations),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(
            select_join_and_union_keep_independent_ordered_bags
        )),
        word: Word::Succeeds,
        input: || (Boundary::Delivered, &[Select, Join, Union]),
        expect: Expect::Value(Cow::Borrowed(&[
            (Select, Observed::Delivered(3, Conformance::Conforms)),
            (Join, Observed::Delivered(3, Conformance::Conforms)),
            (Union, Observed::Delivered(3, Conformance::Conforms)),
        ])),
    },
    Case {
        name: CaseName("a_join_keeps_each_left_row_with_its_right_matches_in_order"),
        word: Word::Succeeds,
        input: || (Boundary::KeyedJoin, &[Join]),
        expect: Expect::Value(Cow::Borrowed(&[(
            Join,
            Observed::Rows(Cow::Borrowed(&[
                joined(10, 20),
                joined(10, 22),
                joined(11, 20),
                joined(11, 22),
                joined(12, 21),
            ])),
        )])),
    },
    Case {
        name: CaseName(stringify!(
            schema_failures_and_emitter_prefix_survive_the_common_path
        )),
        word: Word::Succeeds,
        input: || (Boundary::Invalid, UNARY),
        expect: Expect::Value(Cow::Borrowed(&[
            (Select, PAST_THE_INPUT),
            (Project, PAST_THE_INPUT),
            (SourceMatches, PAST_THE_INPUT),
            (SpanBounds, PAST_THE_INPUT),
        ])),
    },
    Case {
        name: CaseName(stringify!(
            binary_bad_shape_missing_input_and_sink_prefix_remain_observable
        )),
        word: Word::Succeeds,
        input: || (Boundary::Invalid, BINARY),
        expect: Expect::Value(Cow::Borrowed(&[
            (Join, PAST_THE_INPUT),
            (Union, UNLIKE_SHAPES),
        ])),
    },
    Case {
        name: CaseName("a_binary_schema_without_its_second_input_is_refused"),
        word: Word::Succeeds,
        input: || (Boundary::MissingInput, BINARY),
        expect: Expect::Value(Cow::Borrowed(&[
            (
                Join,
                Observed::MissingInput(Some(OperatingError::MissingInput { position: 2 })),
            ),
            (
                Union,
                Observed::MissingInput(Some(OperatingError::MissingInput { position: 2 })),
            ),
        ])),
    },
    Case {
        name: CaseName(stringify!(
            conformance_detects_a_deliberately_dropped_or_reordered_result
        )),
        word: Word::Succeeds,
        input: || (Boundary::Altered, &[Select]),
        expect: Expect::Value(Cow::Borrowed(&[(
            Select,
            Observed::Compared {
                delivered: true,
                dropped: false,
                other: false,
                none: false,
            },
        )])),
    },
    Case {
        name: CaseName(stringify!(
            positive_row_contracts_keep_partial_input_distinction
        )),
        word: Word::Succeeds,
        input: || {
            (
                Boundary::PartialInputs,
                &[Select, Project, Join, Union, SourceMatches, SpanBounds],
            )
        },
        expect: Expect::Value(Cow::Borrowed(&[
            (Select, needs_of_function(ONE_READ)),
            (Project, needs_of_function(ONE_READ)),
            (Join, needs_of_function(TWO_READS)),
            (Union, needs_of_function(TWO_READS)),
            (SourceMatches, needs_of_function(ONE_READ)),
            (SpanBounds, needs_of_function(ONE_READ)),
        ])),
    },
];

/// Runs the boundary over each kind's fixture.
fn subject((boundary, kinds): Input) -> Result<Value, Failure> {
    let observed = kinds
        .iter()
        .map(|&kind| observe(boundary, kind).map(|observed| (kind, observed)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Cow::Owned(observed))
}

/// Runs the boundary over one kind's fixture.
fn observe(boundary: Boundary, kind: OperationKind) -> Result<Observed, Failure> {
    let mut case = fixture(kind);
    let operation = case.operation.clone();
    let delivered = |case: &mut Fixture, operation: &Operation| {
        compare_delivery(operation, &case.inputs, &[], SOURCE, &mut case.symbols).and_then(settled)
    };
    Ok(match boundary {
        Boundary::Delivered => {
            let observed = delivered(&mut case, &operation)?;
            let conforms = conformance(&case.expected, &observed.rows, &case.symbols);
            Observed::Delivered(observed.rows.len(), conforms)
        }
        Boundary::KeyedJoin => {
            case.inputs = vec![
                matrix(&[&[1, 10], &[1, 11], &[2, 12]]),
                matrix(&[&[1, 20], &[2, 21], &[1, 22]]),
            ];
            let observed = delivered(&mut case, &operation)?;
            Observed::Rows(Cow::Owned(
                observed.rows.iter().map(|row| [row[0], row[1]]).collect(),
            ))
        }
        Boundary::Invalid => {
            let mut invalid = operation.clone();
            #[expect(
                clippy::wildcard_enum_match_arm,
                reason = "the rows name only these fixtures; any other operation is left valid, and the row shows it"
            )]
            match &mut invalid {
                Operation::Select { kept, .. } => kept.push(ColumnIndex(99)),
                Operation::Project { columns, .. } => columns[0] = ColumnIndex(99),
                Operation::SourceMatches { span, .. } | Operation::SpanBounds { span, .. } => {
                    *span = ColumnIndex(99);
                }
                Operation::Join { left_on, .. } => left_on[0] = ColumnIndex(99),
                Operation::Union { .. } => {
                    case.inputs[1] = fixture(SpanBounds).inputs.remove(0);
                }
                _ => {}
            }
            let observed = delivered(&mut case, &invalid)?;
            Observed::Refused([
                observed.schema.err(),
                observed.collected.err(),
                observed.stream_error,
            ])
        }
        Boundary::MissingInput => Observed::MissingInput(
            schema_of(
                &operation,
                &[InputShape {
                    kinds: &case.inputs[0].kinds,
                    tree: None,
                }],
                &[],
            )
            .err(),
        ),
        Boundary::Altered => {
            let observed = delivered(&mut case, &operation)?;
            let table = observed.collected?;
            let rows = observed.rows;
            Observed::Compared {
                delivered: same_rows(&table, &rows),
                dropped: same_rows(&table, &rows[1..]),
                other: same_rows(&table, &[vec![n(1)], vec![n(2)], vec![n(2)]]),
                none: same_rows(&table, &[]),
            }
        }
        Boundary::PartialInputs => needs(&operation),
    })
}

/// A number cell.
const fn n(value: i64) -> Cell {
    Cell::Number(Integer(value))
}

/// The needs of a plan of the operation alone, and how a partial read of each relation it reads blocks.
fn needs(operation: &Operation) -> Observed {
    let plan = inputs::plan(vec![operation.clone()]);
    let needs = needs_of(&plan);
    let scoped = scoped_needs(&plan);
    let relations = named(&inputs_of(operation))
        .into_iter()
        .filter_map(|input| match input {
            PlanInput::Relation(relation) => Some(*relation),
            PlanInput::Step(_) | PlanInput::BoundObservation(_) => None,
        })
        .map(|relation| {
            let partial = Partial {
                relation,
                cause: PartialCause("fixture partial".into()),
                lacking: Lacking::Any,
                at: PartialPlace::Repository,
            };
            let free = needs
                .first()
                .is_some_and(|need| blocked(need, &[partial]) == Blocking::Free);
            let read = scoped.reads.iter().any(|read| read.relation == relation);
            (relation, free, read)
        })
        .collect();
    Observed::Needs {
        sets: needs.len(),
        first_empty: needs.first().is_some_and(|need| need.is_empty()),
        relations: Cow::Owned(relations),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
