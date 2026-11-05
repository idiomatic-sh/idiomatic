//! Production boundary negatives and invocation-state lifetime checks.
use super::*;
use crate::application::file_dag::testing::{Held, prepare};
use crate::domain::operating::types::Memo;
use crate::domain::replay::types::Pipe;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Relation};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, RepeatedRows},
    plans::types::Operation,
};

fn fixture() -> (
    CommonOperatorEngine,
    std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
) {
    let engine = CommonOperatorEngine::fixture();
    let owner = {
        let scope = crate::application::file_dag::testing::semantic_scope(&engine).unwrap();
        std::rc::Rc::clone(scope.owner())
    };
    (engine, owner)
}

#[test]
fn malformed_declared_population_is_refused_not_silently_shortened() {
    let tables = [Table {
        kinds: vec![CellKind::Number],
        cells: vec![Value::Number(Integer(1))],
        count: RowCount(2),
    }];
    let op = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![],
    };
    let contract = prepare(&op, &tables, &[]).unwrap();
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let (engine, owner) = fixture();
    let result = engine.collect(
        &owner,
        &contract,
        &[Rows::Computed(&tables[0])],
        &mut context,
        &mut Table::default(),
    );
    assert!(matches!(result, Err(OperatingError::ShapeMismatch)));
}
#[test]
fn zero_width_bags_keep_their_declared_population() {
    let tables = [Table {
        kinds: vec![],
        cells: vec![],
        count: RowCount(3),
    }];
    let op = Operation::Project {
        from: Input::Relation(Relation::Function),
        columns: vec![],
        distinct: RepeatedRows::Kept,
    };
    let contract = prepare(&op, &tables, &[]).unwrap();
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut out = Table::default();
    let (engine, owner) = fixture();
    engine
        .collect(
            &owner,
            &contract,
            &[Rows::Computed(&tables[0])],
            &mut context,
            &mut out,
        )
        .unwrap();
    assert_eq!(out.count.0, 3);
    assert!(out.cells.is_empty());
}
#[test]
fn chained_registered_evaluations_reuse_downstream_invocation_state() {
    let tables = [Table {
        kinds: vec![CellKind::Number],
        cells: (1..=3).map(|n| Value::Number(Integer(n))).collect(),
        count: RowCount(3),
    }];
    let select = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![],
    };
    let project = Operation::Project {
        from: Input::Relation(Relation::Function),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Kept,
    };
    let upstream = prepare(&select, &tables, &[]).unwrap();
    let downstream = prepare(&project, &tables, &[]).unwrap();
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut starts = Vec::new();
    let mut produced = Vec::new();
    let (engine, owner) = fixture();
    let mut feed = Held {
        owner: std::rc::Rc::clone(&owner),
        tables: &tables,
        visits: vec![0],
    };
    engine
        .stream(
            &owner,
            &upstream,
            &mut feed,
            &mut context,
            &mut Room::default(),
            &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                starts.push(each.memo.len());
                each.memo.push(Memo::Holds);
                // A real registered downstream operator runs on every delivered row. Its
                // engine context belongs to this whole upstream invocation, not this call.
                let input = Table {
                    kinds: vec![CellKind::Number],
                    cells: row.to_vec(),
                    count: RowCount(1),
                };
                let mut out = Table::default();
                engine.collect(
                    &owner,
                    &downstream,
                    &[Rows::Computed(&input)],
                    each,
                    &mut out,
                )?;
                produced.extend(out.cells);
                Ok(())
            }),
        )
        .unwrap();
    assert_eq!(starts, vec![0, 1, 2]);
    assert_eq!(produced, tables[0].cells);
    assert_eq!(feed.visits, vec![3]);
}

#[test]
fn partition_observations_keep_large_then_small_peak_and_release() {
    let mut edges = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    for at in 0..32 {
        ryo_values::domain::rows::service::push_row(
            &mut edges,
            &[
                Value::Number(Integer(at)),
                Value::Number(Integer(at + 1)),
                Value::Number(Integer(0)),
            ],
        );
    }
    ryo_values::domain::rows::service::push_row(
        &mut edges,
        &[
            Value::Number(Integer(100)),
            Value::Number(Integer(101)),
            Value::Number(Integer(1)),
        ],
    );
    let operation = Operation::Closure {
        edges: Input::Relation(Relation::Function),
        keys: vec![ColumnIndex(2)],
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
    };
    let contract = prepare(&operation, std::slice::from_ref(&edges), &[]).unwrap();
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut output = Table::default();
    let (engine, owner) = fixture();
    engine
        .collect(
            &owner,
            &contract,
            &[Rows::Computed(&edges)],
            &mut context,
            &mut output,
        )
        .unwrap();
    assert_eq!(output.count.0, 32 * 33 / 2 + 1);
    assert_eq!(scratch.observation.closure_partition_rows, 32);
    assert!(scratch.observation.closure_partition_capacity >= 64);
    assert!(scratch.observation.local_resources_released);
}

#[test]
fn ragged_selected_fallback_cannot_admit_mismatched_projection_kinds() {
    use crate::domain::table_model::types::{SelectedRows, StoredSelection};
    use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
    use std::sync::Arc;
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![3])];
    let selection = Arc::new(StoredSelection {
        relation: Relation::Function,
        rows: Arc::new(vec![0]),
        columns: vec![ColumnIndex(0)],
        kinds: vec![CellKind::Symbol],
        identity: Arc::new(Default::default()),
    });
    let rows = Rows::Selected(SelectedRows {
        source: ColumnsView {
            kinds: &[CellKind::Number, CellKind::Number],
            columns: &columns,
        },
        selection,
    });
    let schema = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    let operation = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![],
    };
    let contract = prepare(&operation, &[schema], &[]).unwrap();
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut out = Table::default();
    let (engine, owner) = fixture();
    assert!(matches!(
        engine.collect(&owner, &contract, &[rows], &mut context, &mut out),
        Err(OperatingError::ShapeMismatch)
    ));
    assert_eq!(out.count.0, 0);
}
