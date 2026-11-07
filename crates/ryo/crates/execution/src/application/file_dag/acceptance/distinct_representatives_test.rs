//! The projected tuple contract retains collision and invalid-cell controls through the capability.
use crate::outbound::derived::interface::DerivedPort as _;
use ryo_derived::domain::contract::types::BuildOutcome;
use ryo_derived::domain::contract::types::BuildWorkspace;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::contract::types::BuildInputs;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_derived::inbound::use_cases::build::interface::BuildUseCase;
use ryo_derived::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_derived::inbound::use_cases::view::interface::ViewUseCase;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::borrow::Cow;

fn representatives(
    source: ColumnsView<'_>,
    columns: &[ColumnIndex],
    positions: &[u32],
) -> Result<Vec<u32>, DerivedFault> {
    let port = std::sync::Arc::new(
        crate::outbound::derived::factory::semantics(),
    );
    let plan = port.prepare(
        DerivedRequest::DistinctRepresentatives {
            columns: columns.to_vec(),
        },
        vec![source.kinds.to_vec()],
    );
    let rows = crate::domain::positions::service_test::SemanticRows(
        crate::domain::table_model::types::Rows::Stored(source),
    );
    let BuildOutcome::Built(payload) = port.build(
        &plan,
        BuildInputs::Distinct {
            rows: &rows,
            positions: ryo_derived::domain::positions::types::RowSlots(positions),
        },
        &mut BuildWorkspace::default(),
        None,
    )?
    else {
        panic!("readable distinct rows")
    };
    Ok(payload
        .share_positions()
        .expect("successful distinct payload")
        .0
        .as_slice()
        .to_vec())
}

/// The rows and the discriminating columns the distinct-representatives view is asked about.
#[derive(Clone, Copy)]
enum Scenario {
    /// Rows that repeat on columns that repeat, one of them an invalid flag cell.
    Duplicates,
    /// A position past the last row of the source.
    PositionPastRows,
    /// A discriminating column the source does not have.
    ColumnPastSource,
    /// Two different rows whose full hashes collide.
    HashCollision,
}
/// The scenario the view is built for.
type Input = Scenario;
/// The positions the view keeps, in order.
type Value = Cow<'static, [u32]>;
/// The fault the view refuses with.
type Failure = DerivedFault;

/// Build the view over the scenario's rows and answer the positions it keeps.
fn subject(scenario: Input) -> Result<Value, Failure> {
    let answer = match scenario {
        Scenario::Duplicates | Scenario::PositionPastRows | Scenario::ColumnPastSource => {
            let columns = [
                FactColumn(vec![7, 7, 7, 7, 7, 8]),
                FactColumn(vec![1, 2, 0, 7, 0, 1]),
                FactColumn(vec![10, 10, 20, 11, 20, 10]),
            ];
            let source = ColumnsView {
                kinds: &[CellKind::Number, CellKind::Flag, CellKind::Symbol],
                columns: &columns,
            };
            let on = [ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)];
            match scenario {
                Scenario::Duplicates => representatives(source, &on, &[0, 1, 2, 3, 4]),
                Scenario::PositionPastRows => representatives(source, &on, &[0, 99]),
                _ => representatives(source, &[ColumnIndex(99)], &[0]),
            }
        }
        Scenario::HashCollision => {
            use ryo_derived::domain::key::service::hash_row;
            use ryo_derived::domain::key::types::On;
            use ryo_relations::domain::relations::types::{Integer, Value};
            let collision = 0x517c_c1b7_2722_0a95_u64.rotate_left(5);
            let columns = [FactColumn(vec![0, 1, 0]), FactColumn(vec![0, collision, 0])];
            let source = ColumnsView {
                kinds: &[CellKind::Number; 2],
                columns: &columns,
            };
            let on = [ColumnIndex(0), ColumnIndex(1)];
            assert_eq!(
                hash_row(
                    &[Value::Number(Integer(0)), Value::Number(Integer(0))],
                    On::Whole
                ),
                hash_row(
                    &[
                        Value::Number(Integer(1)),
                        Value::Number(Integer(collision.cast_signed()))
                    ],
                    On::Whole
                ),
                "actual semantic full-hash collision"
            );
            representatives(source, &on, &[0, 1, 2])
        }
    };
    answer.map(Cow::Owned)
}

/// The projected tuple contract retains collision and invalid-cell controls through the capability.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("distinct_representatives_handle_duplicates_and_invalid_cells"),
        word: Word::Succeeds,
        input: || Scenario::Duplicates,
        expect: Expect::Value(Cow::Borrowed(&[0, 2, 3])),
    },
    Case {
        name: CaseName(
            "distinct_representatives_compare_exact_values_after_a_real_full_hash_collision",
        ),
        word: Word::Succeeds,
        input: || Scenario::HashCollision,
        expect: Expect::Value(Cow::Borrowed(&[0, 1])),
    },
    Case {
        name: CaseName("a_position_past_the_rows_is_a_shape_mismatch"),
        word: Word::Refuses,
        input: || Scenario::PositionPastRows,
        expect: Expect::Error(DerivedFault::ShapeMismatch),
    },
    Case {
        name: CaseName("a_column_past_the_source_is_named_in_the_refusal"),
        word: Word::Refuses,
        input: || Scenario::ColumnPastSource,
        expect: Expect::Error(DerivedFault::NoSuchColumn {
            column: ColumnIndex(99),
        }),
    },
];

/// Run every row the view answers.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// Run every row the view refuses.
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
