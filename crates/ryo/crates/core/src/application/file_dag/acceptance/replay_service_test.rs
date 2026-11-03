//! Scan plus transport/key scenarios migrated from obsolete replay/hash facades.
use crate::application::file_dag::feed::Feed;
use crate::application::file_dag::testing::TestOperator as _;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, InputPlace, OperatorContext, Room, Supply};
use crate::domain::operator::collections::operators::{
    join::item::JoinItem, project::item::ProjectItem,
};
use crate::domain::replay::service::scan;
use crate::domain::replay::types::{Collect, Pipe};
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{RowCount, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::{CellKind, FileId, Integer, Relation, Value};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, JoinColumn, RepeatedRows, Side};
use ryo_values::domain::plans::types::Operation;
const SOURCE: &[u8] = b"replayed";

fn context<T>(run: impl FnOnce(&mut OperatorContext<'_>) -> T) -> T {
    let mut symbols = Symbols::default();
    run(&mut OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    })
}
fn files(count: u32) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::File],
        ..Table::default()
    };
    for file in 0..count {
        push_row(&mut table, &[Value::File(FileId(file))]);
    }
    table
}
struct Held<'f>(&'f Table);
impl<'f> Feed<'f> for Held<'f> {
    fn kinds(&self, _: InputPlace) -> Result<&[CellKind], OperatingError> {
        Ok(&self.0.kinds)
    }
    fn found(&mut self, _: InputPlace, _: RowCount) {}
    fn streams(&self, _: InputPlace) -> Supply {
        Supply::AtHand
    }
    fn stream(
        &mut self,
        _: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        scan(&Rows::Computed(self.0), &mut Vec::new(), context, emit)
    }
    fn hold<'t>(
        &mut self,
        _: InputPlace,
        _: &mut OperatorContext<'_>,
        _: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'f: 't,
    {
        Ok(Rows::Computed(self.0))
    }
}
fn project(column: u32) -> Operation {
    Operation::Project {
        from: Input::Relation(Relation::Function),
        columns: vec![ColumnIndex(column)],
        distinct: RepeatedRows::Kept,
    }
}
#[test]
fn a_scan_passes_on_every_row_of_a_table() {
    context(|context| {
        let from = files(3);
        let mut into = Table::default();
        scan(
            &Rows::Computed(&from),
            &mut Vec::new(),
            context,
            &mut Collect(&mut into),
        )
        .unwrap();
        assert_eq!(into.cells, from.cells);
    });
}
#[test]
fn a_scan_stops_at_the_row_the_taker_refuses() {
    context(|context| {
        let mut calls = 0;
        let mut sink = Pipe(|_: &mut OperatorContext<'_>, _: &[Value]| {
            calls += 1;
            Err(OperatingError::NotRun)
        });
        assert_eq!(
            scan(
                &Rows::Computed(&files(3)),
                &mut Vec::new(),
                context,
                &mut sink
            ),
            Err(OperatingError::NotRun)
        );
        assert_eq!(calls, 1);
    });
}
#[test]
fn the_engine_sink_reads_the_original_source_context() {
    context(|context| {
        let from = files(2);
        let mut observed = Vec::new();
        let mut sink = Pipe(|own: &mut OperatorContext<'_>, row: &[Value]| {
            assert_eq!(own.source.0, SOURCE);
            observed.push(row[0]);
            Ok(())
        });
        ProjectItem
            .stream(
                &project(0),
                &mut Held(&from),
                context,
                &mut Room::default(),
                &mut sink,
            )
            .unwrap();
        assert_eq!(observed, from.cells);
    });
}
#[test]
fn a_real_step_passes_on_the_rows_it_computes() {
    context(|context| {
        let from = files(2);
        let mut into = Table::default();
        ProjectItem
            .stream(
                &project(0),
                &mut Held(&from),
                context,
                &mut Room::default(),
                &mut Collect(&mut into),
            )
            .unwrap();
        assert_eq!(into.cells, from.cells);
    });
}
#[test]
fn rows_the_real_computation_refuses_are_not_passed_on() {
    context(|context| {
        let from = files(2);
        let mut calls = 0;
        let mut sink = Pipe(|_: &mut OperatorContext<'_>, _: &[Value]| {
            calls += 1;
            Ok(())
        });
        let result = ProjectItem.stream(
            &project(99),
            &mut Held(&from),
            context,
            &mut Room::default(),
            &mut sink,
        );
        assert_eq!(
            result,
            Err(OperatingError::Table(TableModelError::NoSuchColumn {
                index: 99
            }))
        );
        assert_eq!(calls, 0);
    });
}
fn indexed_join(key: i64, column: u32) -> Result<Table, OperatingError> {
    let mut left = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    push_row(&mut left, &[Value::Number(Integer(key))]);
    let mut right = Table {
        kinds: vec![CellKind::Number, CellKind::Number],
        ..Table::default()
    };
    for row in [[1, 10], [2, 20], [1, 30]] {
        push_row(&mut right, &row.map(|value| Value::Number(Integer(value))));
    }
    let operation = Operation::Join {
        left: Input::Relation(Relation::Function),
        right: Input::Relation(Relation::Function),
        left_on: vec![ColumnIndex(column)],
        right_on: vec![ColumnIndex(0)],
        output: vec![JoinColumn {
            side: Side::Right,
            column: ColumnIndex(1),
        }],
        filter: vec![],
        view: vec![],
        reach: None,
    };
    context(|context| {
        let mut out = Table::default();
        JoinItem.compute(
            &operation,
            &[Rows::Computed(&left), Rows::Computed(&right)],
            context,
            &mut out,
        )?;
        Ok(out)
    })
}
#[test]
fn a_key_finds_the_rows_that_have_it_in_input_order() {
    assert_eq!(
        indexed_join(1, 0).unwrap().cells,
        [Value::Number(Integer(10)), Value::Number(Integer(30))]
    );
}
#[test]
fn a_key_no_row_has_finds_nothing() {
    assert_eq!(indexed_join(9, 0).unwrap().count.0, 0);
}
#[test]
fn a_join_row_without_its_key_column_is_refused() {
    assert!(matches!(
        indexed_join(1, 3),
        Err(OperatingError::Table(TableModelError::NoSuchColumn {
            index: 3
        }))
    ));
}
#[test]
fn computed_inputs_use_the_common_typed_index_without_a_native_tree_finding() {
    // Replaces the deleted HashItem.applies(None) dispatch-shell assertion with
    // its actual execution behavior. Native node/span routes retain their own tests.
    assert_eq!(
        indexed_join(2, 0).unwrap().cells,
        [Value::Number(Integer(20))]
    );
}
