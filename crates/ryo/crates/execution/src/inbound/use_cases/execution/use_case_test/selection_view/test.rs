//! Projected-row geometry and its exact logical selection output use the real owner.
use super::super::super::{fixtures, types::OperatingError};
use crate::domain::table_model::{
    service::{materialize, read_row},
    types::{Rows, SelectedRows, StoredSelection},
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{CellKind, ColumnsView, FactColumn, Integer, Relation, Value as Cell},
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{
            ColumnIndex, PlanComparison, Input as Source, Operand, QueryLiteral, Test,
        },
        plans::types::Operation,
        rows::types::{RowIndex, Table},
    },
};
use std::sync::Arc;
type Input = bool;
type Value = ();
type Failure = OperatingError;
fn n(value: i64) -> Cell {
    Cell::Number(Integer(value))
}
fn subject(ragged: Input) -> Result<Value, Failure> {
    let mut physical = [
        vec![10, 20, 30],
        vec![1, 0, 1],
        vec![100, 200, 300],
        vec![1000, 2000, 3000],
    ]
    .map(|values| {
        FactColumn(
            values
                .into_iter()
                .map(|value| to_bits(n(value)).0)
                .collect(),
        )
    });
    if ragged {
        physical[3].0.truncate(2);
    }
    let selected = Rows::Selected(SelectedRows {
        source: ColumnsView {
            kinds: &[CellKind::Number; 4],
            columns: &physical,
        },
        selection: Arc::new(StoredSelection {
            relation: Relation::Function,
            rows: Arc::new(vec![2, 0, 2, 1]),
            columns: vec![
                ColumnIndex(2),
                ColumnIndex(1),
                ColumnIndex(0),
                ColumnIndex(2),
                ColumnIndex(3),
            ],
            kinds: vec![CellKind::Number; 5],
            identity: Arc::new(Default::default()),
        }),
    });
    // Generic representation data never masquerades as a catalog provider.
    // The normal owner receives its logical numeric rows as a real hosted relation.
    let mut logical = Table::default();
    materialize(&selected, &mut logical, &mut vec![]);
    for projected in [true, false] {
        let operation = Operation::Select {
            from: Source::Relation(Relation::Function),
            comparisons: vec![PlanComparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Literal(QueryLiteral::Number(Integer(1))),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            }],
            kept: if projected {
                vec![ColumnIndex(3), ColumnIndex(0), ColumnIndex(3)]
            } else {
                vec![]
            },
        };
        let (actual, _) =
            fixtures::table_request(operation, &[&logical], vec![], &mut Symbols::default())?;
        let full = if ragged {
            vec![vec![100, 1, 10, 100, 1000]]
        } else {
            vec![
                vec![300, 1, 30, 300, 3000],
                vec![100, 1, 10, 100, 1000],
                vec![300, 1, 30, 300, 3000],
            ]
        };
        let expected = if projected {
            if ragged {
                vec![vec![100, 100, 100]]
            } else {
                vec![
                    vec![300, 300, 300],
                    vec![100, 100, 100],
                    vec![300, 300, 300],
                ]
            }
        } else {
            full
        };
        assert_eq!(actual.count.0, expected.len());
        assert_eq!(
            actual.kinds,
            vec![CellKind::Number; if projected { 3 } else { 5 }]
        );
        assert_eq!(
            actual.cells,
            expected.into_iter().flatten().map(n).collect::<Vec<_>>()
        );
    }
    let Rows::Selected(mut missing) = selected else {
        unreachable!()
    };
    missing.selection = Arc::new(StoredSelection {
        relation: missing.selection.relation,
        rows: Arc::new(vec![99]),
        columns: missing.selection.columns.clone(),
        kinds: missing.selection.kinds.clone(),
        identity: Arc::clone(&missing.selection.identity),
    });
    assert!(
        read_row(&Rows::Selected(missing), RowIndex(0), &mut vec![]).is_none(),
        "the actual representation refuses a missing physical row before a predicate can receive it"
    );
    Ok(())
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reordered_duplicate_positions_keep_exact_projected_rows"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("ragged_selected_rows_omit_only_unavailable_physical_rows"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(()),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
