//! Exact endpoint projection for repeated rows and invalid spans.
use crate::application::file_dag::feed::Feed;

use crate::application::file_dag::testing::TestOperator as _;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, InputPlace, OperatorContext, Room, Supply};
use crate::domain::operator::collections::operators::span_bounds::item::SpanBoundsItem;
use crate::domain::replay::types::Pipe;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::constants::NO_SPAN;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Integer, Offset, Relation, Span, Value,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};
use ryo_values::domain::plans::types::{Operation, QueryOperator};

fn span(start: u32, end: u32) -> Span {
    Span {
        start: Offset(start),
        end: Offset(end),
    }
}

struct Held<'f>(
    &'f Table,
    std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
);
fn held(table: &Table) -> Held<'_> {
    Held(
        table,
        std::rc::Rc::new(crate::application::file_dag::relations::RelationOwner::new(
            true,
            crate::outbound::derived::factory::test_port(),
        )),
    )
}
impl<'f> Feed<'f> for Held<'f> {
    fn relation_owner(
        &self,
    ) -> std::rc::Rc<crate::application::file_dag::relations::RelationOwner> {
        std::rc::Rc::clone(&self.1)
    }
    fn origin(
        &self,
        which: InputPlace,
    ) -> Result<crate::application::file_dag::feed::InputOrigin, OperatingError> {
        crate::application::file_dag::testing::external_origin(which, 1)
    }
    fn publication(
        &self,
        which: InputPlace,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, OperatingError>
    {
        crate::application::file_dag::testing::no_publication(which, 1)
    }
    fn kinds(&self, which: InputPlace) -> Result<&[CellKind], OperatingError> {
        crate::application::file_dag::testing::supplied(which, 1, Supply::AtHand)?;
        Ok(&self.0.kinds)
    }
    fn streams(&self, which: InputPlace) -> Result<Supply, OperatingError> {
        crate::application::file_dag::testing::supplied(which, 1, Supply::AtHand)
    }
    fn found(&mut self, _: InputPlace, _: RowCount) {}
    fn stream(
        &mut self,
        _: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        for row in self.0.cells.chunks_exact(1) {
            emit.row(context, row)?;
        }
        Ok(())
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

fn evaluate(spans: &[Span]) -> Result<Vec<Value>, OperatingError> {
    let mut input = Table {
        kinds: vec![CellKind::Span],
        ..Table::default()
    };
    for &span in spans {
        push_row(&mut input, &[Value::Span(span)]);
    }
    let operation = Operation::SpanBounds {
        from: PlanInput::Relation(Relation::Function),
        span: ColumnIndex(0),
    };
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b"abcdef"),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut output = Table::default();
    let computed = SpanBoundsItem.compute(
        &operation,
        &[Rows::Computed(&input)],
        &mut context,
        &mut output,
    );
    let mut streamed = Vec::new();
    let streaming = SpanBoundsItem.stream(
        &operation,
        &mut held(&input),
        &mut context,
        &mut Room::default(),
        &mut Pipe(|_: &mut OperatorContext<'_>, row: &[Value]| {
            streamed.extend_from_slice(row);
            Ok(())
        }),
    );
    assert_eq!(computed, streaming);
    computed?;
    assert_eq!(output.cells, streamed);
    assert_eq!(
        output.kinds,
        vec![CellKind::Span, CellKind::Number, CellKind::Number]
    );
    Ok(output.cells)
}

#[test]
fn checked_bounds_preserve_duplicate_rows_and_order() {
    assert_eq!(
        evaluate(&[span(2, 4), span(2, 4), span(0, 0)]),
        Ok(vec![
            Value::Span(span(2, 4)),
            Value::Number(Integer(2)),
            Value::Number(Integer(4)),
            Value::Span(span(2, 4)),
            Value::Number(Integer(2)),
            Value::Number(Integer(4)),
            Value::Span(span(0, 0)),
            Value::Number(Integer(0)),
            Value::Number(Integer(0)),
        ])
    );
}

#[test]
fn absent_and_reversed_spans_refuse_before_row_output() {
    for invalid in [NO_SPAN, span(4, 3)] {
        assert_eq!(evaluate(&[invalid]), Err(OperatingError::InvalidSpanBounds));
    }
    assert_eq!(
        evaluate(&[span(0, 7)]),
        Ok(vec![
            Value::Span(span(0, 7)),
            Value::Number(Integer(0)),
            Value::Number(Integer(7)),
        ]),
        "pure endpoint extraction does not claim the source bytes of a cross-file row"
    );
}

#[test]
fn equal_offsets_from_distinct_files_keep_both_file_identities() {
    let mut input = Table {
        kinds: vec![CellKind::File, CellKind::Span],
        ..Table::default()
    };
    for file in [FileId(1), FileId(2)] {
        push_row(&mut input, &[Value::File(file), Value::Span(span(2, 4))]);
    }
    let operation = Operation::SpanBounds {
        from: PlanInput::Relation(Relation::Function),
        span: ColumnIndex(1),
    };
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut output = Table::default();
    SpanBoundsItem
        .compute(
            &operation,
            &[Rows::Computed(&input)],
            &mut context,
            &mut output,
        )
        .expect("pure cross-file projection");
    assert_eq!(
        output.cells,
        vec![
            Value::File(FileId(1)),
            Value::Span(span(2, 4)),
            Value::Number(Integer(2)),
            Value::Number(Integer(4)),
            Value::File(FileId(2)),
            Value::Span(span(2, 4)),
            Value::Number(Integer(2)),
            Value::Number(Integer(4)),
        ]
    );
}

#[test]
fn contract() {
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::SpanBounds,
    );
}

use ryo_values::domain::rows::types::{RowCount, Table};
