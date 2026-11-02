//! Test transport around the production contract preparation and common executor.
use super::execution::CommonOperatorEngine;
use crate::domain::features::service::kind_of;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, Feed, InputPlace, OperatorContext, Room, Supply},
};
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::operator::{contract::types::OperatorContract, service::registered};
use crate::domain::replay::types::Pipe;
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::{
    service::read_row,
    types::{ParameterBinding, RowCount, Rows, Table},
};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::{
    relations::types::{CellKind, Value},
    symbols::types::Symbols,
};
use ryo_values::domain::plans::types::Operation;

pub(crate) struct Held<'a> {
    pub(crate) tables: &'a [Table],
    pub(crate) visits: Vec<usize>,
}
impl<'f> Feed<'f> for Held<'f> {
    fn kinds(&self, place: InputPlace) -> Result<&[CellKind], OperatingError> {
        self.tables
            .get(place.0)
            .map(|table| table.kinds.as_slice())
            .ok_or(OperatingError::MissingInput {
                position: place.0 + 1,
            })
    }
    fn streams(&self, _: InputPlace) -> Supply {
        Supply::AtHand
    }
    fn found(&mut self, _: InputPlace, _: RowCount) {}
    fn stream(
        &mut self,
        place: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let table = self
            .tables
            .get(place.0)
            .ok_or(OperatingError::MissingInput {
                position: place.0 + 1,
            })?;
        let mut buffer = Vec::new();
        for at in 0..table.count.0 {
            if let Some(row) = read_row(&Rows::Computed(table), RowIndex(at), &mut buffer) {
                self.visits[place.0] += 1;
                emit.row(context, row)?;
            }
        }
        Ok(())
    }
    fn hold<'t>(
        &mut self,
        place: InputPlace,
        _: &mut OperatorContext<'_>,
        _: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'f: 't,
    {
        let table = self
            .tables
            .get(place.0)
            .ok_or(OperatingError::MissingInput {
                position: place.0 + 1,
            })?;
        self.visits[place.0] += table.count.0;
        Ok(Rows::Computed(table))
    }
}
pub(crate) struct Observation {
    pub(crate) schema: Result<Vec<CellKind>, OperatingError>,
    pub(crate) collected: Result<Table, OperatingError>,
    pub(crate) stream_error: Option<OperatingError>,
    pub(crate) rows: Vec<Vec<Value>>,
    pub(crate) visits: Vec<usize>,
    pub(crate) scratch_cells: usize,
}
pub(crate) fn prepare(
    operation: &Operation,
    tables: &[Table],
    bindings: &[ParameterBinding],
) -> Result<OperatorContract, OperatingError> {
    let schemas = tables
        .iter()
        .map(|table| InputSchema {
            kinds: &table.kinds,
            tree: None,
        })
        .collect::<Vec<_>>();
    registered(kind_of(operation)).prepare(operation, &schemas, bindings)
}
/// Both modes call the same real production engine, including all resource acquisition.
pub(crate) fn execute(
    operation: &Operation,
    tables: &[Table],
    bindings: &[ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
    stop_after: Option<usize>,
) -> Result<Observation, OperatingError> {
    let contract = prepare(operation, tables, bindings)?;
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings,
        symbols: symbols.into(),
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut out = Table::default();
    let inputs = tables.iter().map(Rows::Computed).collect::<Vec<_>>();
    let collected = CommonOperatorEngine
        .collect(&contract, &inputs, &mut context, &mut out)
        .map(|()| out);
    let mut room = Room::default();
    let mut rows = Vec::new();
    let mut feed = Held {
        tables,
        visits: vec![0; tables.len()],
    };
    let streamed = CommonOperatorEngine.stream(
        &contract,
        &mut feed,
        &mut context,
        &mut room,
        &mut Pipe(|_: &mut OperatorContext<'_>, row: &[Value]| {
            if stop_after == Some(rows.len()) {
                return Err(OperatingError::WrongOperation);
            }
            rows.push(row.to_vec());
            Ok(())
        }),
    );
    Ok(Observation {
        schema: contract.schema,
        collected,
        stream_error: streamed.err(),
        rows,
        visits: feed.visits,
        scratch_cells: room.scratch.observation.row_capacity
            + room.scratch.observation.other_capacity,
    })
}
/// The ordered-bag check deliberately accepts independent observations, enabling a negative guard.
pub(crate) fn same_rows(table: &Table, rows: &[Vec<Value>]) -> bool {
    table.count.0 == rows.len()
        && rows.iter().all(|row| row.len() == table.kinds.len())
        && table.cells == rows.iter().flatten().copied().collect::<Vec<_>>()
}

/// Compatibility of test call syntax only: every assertion reaches the production executor.
/// No compute/stream primitive implementation survives behind this extension.
pub(crate) trait TestOperator:
    crate::domain::operator::collections::operators::interface::OperatorsItem
{
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let schemas = inputs
            .iter()
            .map(|rows| InputSchema {
                kinds: crate::domain::table_model::service::kinds_of(rows),
                tree: match rows {
                    Rows::Tree(tree) => Some(tree.form),
                    _ => None,
                },
            })
            .collect::<Vec<_>>();
        let contract = self.prepare(operation, &schemas, context.bindings)?;
        CommonOperatorEngine.collect(&contract, inputs, context, out)
    }
    fn stream(
        &self,
        operation: &Operation,
        feed: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let count = ryo_values::domain::plans::service::named(
            &ryo_values::domain::plans::service::inputs_of(operation),
        )
        .len()
            + usize::from(matches!(operation, Operation::Hosted { .. }));
        let kinds = (0..count)
            .map(|at| feed.kinds(InputPlace(at)).map(<[CellKind]>::to_vec))
            .collect::<Result<Vec<_>, _>>()?;
        let schemas = kinds
            .iter()
            .map(|kinds| InputSchema { kinds, tree: None })
            .collect::<Vec<_>>();
        let contract = self.prepare(operation, &schemas, context.bindings)?;
        CommonOperatorEngine.stream(&contract, feed, context, room, emit)
    }
}
impl<T: crate::domain::operator::collections::operators::interface::OperatorsItem + ?Sized>
    TestOperator for T
{
}

/// Native fixtures supply representation metadata from rows they already own, without extra Feed demand.
pub(crate) fn stream_with_inputs(
    item: &dyn crate::domain::operator::collections::operators::interface::OperatorsItem,
    operation: &Operation,
    inputs: &[Rows<'_>],
    feed: &mut dyn Feed<'_>,
    context: &mut OperatorContext<'_>,
    room: &mut Room,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let schemas = inputs
        .iter()
        .map(|rows| InputSchema {
            kinds: crate::domain::table_model::service::kinds_of(rows),
            tree: match rows {
                Rows::Tree(tree) => Some(tree.form),
                _ => None,
            },
        })
        .collect::<Vec<_>>();
    let contract = item.prepare(operation, &schemas, context.bindings)?;
    CommonOperatorEngine.stream(&contract, feed, context, room, emit)
}

/// Existing borrowed test worlds keep request metadata for their fixture lifetime.
/// Production owns this value on the request stack and never calls this helper.
pub(crate) fn request_fixture(
    plans: &[ryo_values::domain::plans::types::Plan],
    bindings: &[Vec<ParameterBinding>],
    hosted: &[ryo_values::domain::hosted::types::HostRelation],
) -> std::sync::Arc<crate::domain::operator::contract::types::RequestOperators> {
    std::sync::Arc::new(
        crate::domain::operator::contract::types::RequestOperators::prepare(
            plans, bindings, hosted,
        )
        .expect("fixture metadata"),
    )
}
