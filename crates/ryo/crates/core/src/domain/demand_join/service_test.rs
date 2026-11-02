//! Physical demand joins preserve the ordinary closure/join's exact ordered bags.

use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::closure::item::ClosureItem;
use crate::domain::operator::collections::operators::interface::OperatorsItem as _;
use crate::domain::operator::collections::operators::join::item::JoinItem;
use crate::domain::operator::collections::operators::project::item::ProjectItem;
use crate::domain::stored_indexes::service::{LIMIT, new};
use crate::domain::stored_indexes::types::FileIndexes;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{Rows, Table};
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input, JoinColumn, ReachLookup, RepeatedRows, Side,
};
use ryo_values::domain::plans::types::Operation;

const SOURCE: Input = Input::Relation(Relation::Function);

fn table(rows: &[&[i64]], width: usize) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; width],
        ..Default::default()
    };
    for row in rows {
        push_row(
            &mut table,
            &row.iter()
                .map(|value| Value::Number(Integer(*value)))
                .collect::<Vec<_>>(),
        );
    }
    table
}

fn compute(
    operation: &Operation,
    left: &Rows<'_>,
    right: &Rows<'_>,
    cache: Option<&mut FileIndexes>,
) -> Result<Table, OperatingError> {
    let mut symbols = Symbols::default();
    let mut out = Table::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        stored_indexes: cache,
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut Default::default(),
    };
    JoinItem.compute(
        operation,
        &[left.clone(), right.clone()],
        &mut context,
        &mut out,
    )?;
    Ok(out)
}

#[test]
fn demanded_partitioned_projected_keys_equal_ordinary_join_in_exact_row_order() {
    // BFS encounter [9,2] must become closure's typed sorted [2,9]. Partitions reuse endpoint IDs.
    let columns = [
        FactColumn(vec![1, 1, 9, 2, 1, 90]),
        FactColumn(vec![9, 2, 1, 2, 8, 91]),
        FactColumn(vec![7, 7, 7, 7, 8, 9]),
    ];
    let edges = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 3],
        columns: &columns,
    });
    let left = table(&[&[1, 7], &[1, 7], &[1, 8], &[999, 7], &[9, 7]], 2);
    let reach = ReachLookup {
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
        keys: vec![ColumnIndex(2)],
        columns: vec![
            ColumnIndex(2),
            ColumnIndex(1),
            ColumnIndex(0),
            ColumnIndex(2),
        ],
    };
    let physical = Operation::Join {
        left: SOURCE,
        right: SOURCE,
        left_on: vec![ColumnIndex(1), ColumnIndex(0)],
        right_on: vec![ColumnIndex(2), ColumnIndex(1)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(3),
            },
        ],
        filter: vec![],
        view: vec![],
        reach: Some(reach.clone()),
    };
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        stored_indexes: None,
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut Default::default(),
    };
    let mut closure = Table::default();
    ClosureItem
        .compute(
            &Operation::Closure {
                edges: SOURCE,
                source: reach.source,
                destination: reach.destination,
                keys: reach.keys.clone(),
            },
            &[edges.clone()],
            &mut context,
            &mut closure,
        )
        .expect("closure");
    let mut projected = Table::default();
    ProjectItem
        .compute(
            &Operation::Project {
                from: SOURCE,
                columns: reach.columns,
                distinct: RepeatedRows::Kept,
            },
            &[Rows::Computed(&closure)],
            &mut context,
            &mut projected,
        )
        .expect("projection");
    let mut ordinary = physical.clone();
    if let Operation::Join { reach, .. } = &mut ordinary {
        *reach = None;
    }
    let expected = compute(
        &ordinary,
        &Rows::Computed(&left),
        &Rows::Computed(&projected),
        None,
    )
    .expect("ordinary join");
    assert!(expected.count.0 > 4);
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("cache");
    cache.graph_enabled = true;
    assert_eq!(
        compute(&physical, &Rows::Computed(&left), &edges, Some(&mut cache)).expect("compact"),
        expected
    );
    assert_eq!(
        compute(&physical, &Rows::Computed(&left), &edges, Some(&mut cache)).expect("second rule"),
        expected
    );
    assert_eq!(cache.graph_counts.builds, 1);
    assert_eq!(cache.graph_counts.reuses, 1);
    assert!(cache.graph_counts.memo_hits > 0);
    assert_eq!(
        compute(&physical, &Rows::Computed(&left), &edges, None).expect("ordinary fallback"),
        expected
    );
    // Empty output preserves the exact multiplicity, not a distinct row set.
    let mut zero = physical.clone();
    if let Operation::Join { output, .. } = &mut zero {
        output.clear();
    }
    let zero =
        compute(&zero, &Rows::Computed(&left), &edges, Some(&mut cache)).expect("zero columns");
    assert!(zero.kinds.is_empty());
    assert!(zero.cells.is_empty());
    assert_eq!(zero.count, expected.count);
}

#[test]
fn residual_destination_keys_missing_roots_and_empty_left_are_preserved() {
    let columns = [FactColumn(vec![1, 1, 2, 9]), FactColumn(vec![9, 2, 3, 1])];
    let edges = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    });
    let operation = Operation::Join {
        left: SOURCE,
        right: SOURCE,
        left_on: vec![ColumnIndex(0), ColumnIndex(1), ColumnIndex(0)],
        right_on: vec![ColumnIndex(0), ColumnIndex(1), ColumnIndex(0)],
        output: vec![JoinColumn {
            side: Side::Right,
            column: ColumnIndex(1),
        }],
        filter: vec![],
        view: vec![],
        reach: Some(ReachLookup {
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            keys: vec![],
            columns: vec![ColumnIndex(0), ColumnIndex(1)],
        }),
    };
    let left = table(&[&[1, 3], &[1, 4], &[999, 3], &[1, 3]], 2);
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("cache");
    cache.graph_enabled = true;
    let actual =
        compute(&operation, &Rows::Computed(&left), &edges, Some(&mut cache)).expect("demand");
    assert_eq!(
        actual.cells,
        [Value::Number(Integer(3)), Value::Number(Integer(3))]
    );
    let empty = table(&[], 2);
    let builds = cache.graph_counts.builds;
    assert_eq!(
        compute(
            &operation,
            &Rows::Computed(&empty),
            &edges,
            Some(&mut cache)
        )
        .expect("empty")
        .count
        .0,
        0
    );
    assert_eq!(
        cache.graph_counts.builds, builds,
        "no adjacency acquisition without actual LHS"
    );
}
