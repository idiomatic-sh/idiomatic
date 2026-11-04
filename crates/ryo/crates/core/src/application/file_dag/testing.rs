//! Test transport around the production contract preparation and common executor.
use super::execution::CommonOperatorEngine;
use crate::application::file_dag::feed::Feed;
use crate::domain::features::service::kind_of;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, InputPlace, OperatorContext, Room, SemanticLookup, Supply},
};
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::operator::{contract::types::OperatorContract, service::registered};
use crate::domain::replay::types::Pipe;

use crate::domain::table_model::{
    service::read_row,
    types::{ParameterBinding, Rows},
};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plans::types::Operation;
/// Complete computed fixture rows for portable derived semantics controls.
pub(crate) struct CompleteTableRows<'a>(pub &'a ryo_values::domain::rows::types::Table);
impl ryo_derived::domain::contract::types::RowCells for CompleteTableRows<'_> {
    fn bulk(&self) -> Option<ryo_derived::domain::contract::types::BulkRows<'_>> {
        Some(ryo_derived::domain::contract::types::BulkRows::Computed(
            self.0,
        ))
    }
    fn validate(&self) -> Result<(), ryo_derived::domain::contract::types::DerivedFault> {
        (self.0.count.0.checked_mul(self.0.kinds.len()) == Some(self.0.cells.len()))
            .then_some(())
            .ok_or(ryo_derived::domain::contract::types::DerivedFault::ShapeMismatch)
    }
    fn kinds(&self) -> &[CellKind] {
        &self.0.kinds
    }
    fn count(&self) -> usize {
        self.0.count.0
    }
    fn cell(
        &self,
        row: usize,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Option<Value> {
        crate::domain::table_model::service::row_in(
            self.0,
            ryo_values::domain::rows::types::RowIndex(row),
        )?
        .get(column.0 as usize)
        .copied()
    }
}

/// Explicit outer fixture scope. All dependency tests use the actual scheduler,
/// required executor/owner and generation constructor; no fallback owner is minted.
pub(crate) struct FileScope<'a> {
    pub(crate) scope: super::scope::ExecutionScope<'a>,
    pub(crate) execution: std::cell::RefCell<super::types::FileDagExecution>,
}
impl<'a> FileScope<'a> {
    pub(crate) fn new(
        graph: std::sync::Arc<crate::domain::file_program::types::RequestGraph>,
        executor: &'a dyn super::executor::PrimitiveExecutor,
        active: impl Fn(usize) -> bool,
        complete: &[ryo_relations::domain::relations::types::Relation],
        partials: &[crate::domain::coverage::types::Partial],
    ) -> Result<Self, OperatingError> {
        let scope = super::scope::ExecutionScope::new(
            std::sync::Arc::clone(&graph),
            super::scope::ScopePhase::File(ryo_relations::domain::relations::types::FileId(0)),
            true,
            executor,
        )?;
        let schedule =
            super::activation::types::ActivationCache::default().activate(&graph, active, complete);
        let completion = scope.completion(
            crate::domain::coverage::types::RuleIndex(0),
            super::scope::ScopePartials::File {
                evaluation: partials,
                publication: partials,
            },
        )?;
        let execution = std::cell::RefCell::new(super::types::FileDagExecution::new_for_schedule(
            &schedule,
            std::rc::Rc::clone(scope.owner()),
            completion,
        ));
        Ok(Self { scope, execution })
    }
}
/// Standalone borrowed-row semantic oracles have an explicit empty request scope:
/// no graph dependency is evaluated and no production owner/default is bypassed.
pub(super) fn semantic_scope<'a>(
    executor: &'a dyn super::executor::PrimitiveExecutor,
) -> Result<super::scope::ExecutionScope<'a>, OperatingError> {
    let request = crate::domain::operator::contract::types::RequestOperators::prepare(
        &[],
        &[],
        &[],
        executor.derived_port(),
    )?;
    let graph = std::sync::Arc::new(crate::domain::file_program::service::compile_prepared(
        &[],
        &request,
    ));
    super::scope::ExecutionScope::new(
        graph,
        super::scope::ScopePhase::File(ryo_relations::domain::relations::types::FileId(0)),
        false,
        executor,
    )
}
/// Real required raw-order capability for bounded semantic fixtures only.
pub(crate) fn raw_scope()
-> Result<impl crate::domain::operator::contract::derived::ScopedDerived, OperatingError> {
    let scope = semantic_scope(super::execution::CommonOperatorEngine::fixture_ref())?;
    Ok(super::derived::owner::raw_scope::RawScope::new(
        std::rc::Rc::clone(scope.owner()),
    ))
}
pub(crate) struct Held<'a> {
    pub(crate) owner: std::rc::Rc<super::relations::RelationOwner>,
    pub(crate) tables: &'a [Table],
    pub(crate) visits: Vec<usize>,
}
/// Test tables are explicitly injected from outside the graph, with no
/// generation-scoped publication. Every fixture still checks its input arity.
pub(crate) fn external_origin(
    place: InputPlace,
    count: usize,
) -> Result<super::feed::InputOrigin, OperatingError> {
    checked(place, count)?;
    Ok(super::feed::InputOrigin::External)
}
pub(crate) fn no_publication(
    place: InputPlace,
    count: usize,
) -> Result<Option<super::relations::PublishedInput>, OperatingError> {
    checked(place, count)?;
    Ok(None)
}
pub(crate) fn supplied(
    place: InputPlace,
    count: usize,
    supply: Supply,
) -> Result<Supply, OperatingError> {
    checked(place, count)?;
    Ok(supply)
}
fn checked(place: InputPlace, count: usize) -> Result<(), OperatingError> {
    (place.0 < count)
        .then_some(())
        .ok_or(OperatingError::MissingInput {
            position: place.0 + 1,
        })
}
impl<'f> Feed<'f> for Held<'f> {
    fn relation_owner(&self) -> std::rc::Rc<super::relations::RelationOwner> {
        std::rc::Rc::clone(&self.owner)
    }
    fn kinds(&self, place: InputPlace) -> Result<&[CellKind], OperatingError> {
        self.tables
            .get(place.0)
            .map(|table| table.kinds.as_slice())
            .ok_or(OperatingError::MissingInput {
                position: place.0 + 1,
            })
    }
    fn origin(&self, place: InputPlace) -> Result<super::feed::InputOrigin, OperatingError> {
        external_origin(place, self.tables.len())
    }
    fn publication(
        &self,
        place: InputPlace,
    ) -> Result<Option<super::relations::PublishedInput>, OperatingError> {
        no_publication(place, self.tables.len())
    }
    fn streams(&self, place: InputPlace) -> Result<Supply, OperatingError> {
        supplied(place, self.tables.len(), Supply::AtHand)
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
    super::derived::prepare::operator(
        registered(kind_of(operation)),
        operation,
        &schemas,
        bindings,
    )
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
    execute_with_semantic(
        operation, tables, bindings, source, symbols, stop_after, None,
    )
}

pub(crate) fn execute_with_semantic(
    operation: &Operation,
    tables: &[Table],
    bindings: &[ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
    stop_after: Option<usize>,
    semantic: Option<&dyn SemanticLookup>,
) -> Result<Observation, OperatingError> {
    execute_with_port(
        operation,
        tables,
        bindings,
        source,
        symbols,
        stop_after,
        semantic,
        crate::outbound::operators::factory::test_port(),
    )
}
pub(crate) fn execute_with_port(
    operation: &Operation,
    tables: &[Table],
    bindings: &[ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
    stop_after: Option<usize>,
    semantic: Option<&dyn SemanticLookup>,
    port: &dyn crate::outbound::operators::interface::OperatorsPort,
) -> Result<Observation, OperatingError> {
    let engine = CommonOperatorEngine::new(port, crate::outbound::derived::factory::test_port());
    let scope = semantic_scope(&engine)?;
    let contract = prepare(operation, tables, bindings)?;
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic,
        prepared_kinds: None,
        prepared_matchers: None,
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
    let collected = engine
        .collect(scope.owner(), &contract, &inputs, &mut context, &mut out)
        .map(|()| out);
    let mut room = Room::default();
    let mut rows = Vec::new();
    let mut feed = Held {
        owner: std::rc::Rc::clone(scope.owner()),
        tables,
        visits: vec![0; tables.len()],
    };
    let streamed = engine.stream(
        scope.owner(),
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
        let contract =
            super::derived::prepare::operator(self, operation, &schemas, context.bindings)?;
        let engine = CommonOperatorEngine::fixture();
        let scope = semantic_scope(&engine)?;
        engine.collect(scope.owner(), &contract, inputs, context, out)
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
        let contract =
            super::derived::prepare::operator(self, operation, &schemas, context.bindings)?;
        let owner = feed.relation_owner();
        let engine = CommonOperatorEngine::new(
            crate::outbound::operators::factory::test_port(),
            owner.derived.semantic_port(),
        );
        engine.stream(&owner, &contract, feed, context, room, emit)
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
    let contract = super::derived::prepare::operator(item, operation, &schemas, context.bindings)?;
    let owner = feed.relation_owner();
    let engine = CommonOperatorEngine::new(
        crate::outbound::operators::factory::test_port(),
        owner.derived.semantic_port(),
    );
    engine.stream(&owner, &contract, feed, context, room, emit)
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
            plans,
            bindings,
            hosted,
            crate::outbound::derived::factory::test_port(),
        )
        .expect("fixture metadata"),
    )
}

/// Test request metadata bound to the same semantic port as its required executor.
pub(crate) fn request_for_executor(
    executor: &dyn super::executor::PrimitiveExecutor,
    plans: &[ryo_values::domain::plans::types::Plan],
    bindings: &[Vec<ParameterBinding>],
    hosted: &[ryo_values::domain::hosted::types::HostRelation],
) -> std::sync::Arc<crate::domain::operator::contract::types::RequestOperators> {
    std::sync::Arc::new(
        crate::domain::operator::contract::types::RequestOperators::prepare(
            plans,
            bindings,
            hosted,
            executor.derived_port(),
        )
        .expect("fixture metadata"),
    )
}

use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
