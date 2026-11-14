//! Strategy inputs and the one real semantic dependency whose answers are refused.
use super::super::capability::fixtures::{REFUSAL, Witness};
use super::super::cases::fixtures::{node, table_of, tree_rows, wide_tree};
use super::super::*;
#[cfg(test)]
use fixtures::execution::*;
#[cfg(test)]
use fixtures::provider::*;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_cells::types::TreeForm;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Flag, Integer, Offset, Relation, Span, Value as Cell},
};
use ryo_relations::{domain::relations::types::NodeId, domain::terms::types::Symbol};
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::rows::types::{RowCount, Table};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Counts the ordered-existence meaning and refuses it on request; every other call is real.
#[derive(Debug, Default)]
pub(super) struct OrderedPort {
    pub(super) calls: AtomicUsize,
    pub(super) refuse: bool,
}
impl crate::outbound::operators::interface::OperatorsPort for OrderedPort {
    fn plan(
        &self,
        program: &ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram<'_>,
    ) -> Result<
        ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        ryo_operators::inbound::use_cases::run::fixtures::planned(program)
    }
    fn run(
        &self,
        plan: &ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
        node: ryo_operators::domain::physical_plan::types::PlanNodeId,
        buffers: &mut ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers,
        inputs: &dyn ryo_operators::domain::physical_plan::types::PlanInputs,
        resources: &mut dyn ryo_operators::domain::physical_plan::types::Resources,
    ) -> Result<(), ryo_operators::domain::contract::types::PortableOperatorFault> {
        ryo_operators::inbound::use_cases::run::fixtures::ran(
            plan, node, buffers, inputs, resources,
        )
    }
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if matches!(call, OperatorCall::OrderedExistence { .. }) {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if self.refuse {
                return Err(PortableOperatorFault::NoSuchColumn {
                    index: ryo_values::domain::plan_parts::types::ColumnIndex(REFUSAL),
                });
            }
        }
        crate::application::file_dag::fixtures::operators().operators(call)
    }
}
/// Three source rows whose numbers are 1, 3 and 2, with the strategy fixture's own schema.
pub(super) fn ordered_sources() -> [Provider<'static>; 2] {
    let (_, schema) = super::super::super::route_controls::fixtures::ordered_strategy_fixture();
    let cells = [1, 3, 2]
        .into_iter()
        .flat_map(|number| {
            schema.iter().map(move |kind| match kind {
                CellKind::Node => Cell::Node(NodeId(0)),
                CellKind::Symbol => Cell::Symbol(Symbol(0)),
                CellKind::File => Cell::File(FileId(0)),
                CellKind::Span => Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                CellKind::Number => Cell::Number(Integer(number)),
                CellKind::Flag => Cell::Flag(Flag::Raised),
            })
        })
        .collect();
    let source = Table {
        kinds: schema,
        cells,
        count: RowCount(3),
    };
    let unused = Table {
        kinds: kinds(Relation::Function).to_vec(),
        cells: vec![],
        count: RowCount(0),
    };
    [
        stored(Relation::SourceLocation, source),
        stored(Relation::Function, unused),
    ]
}

pub(super) fn cancelled() -> Faults {
    Faults {
        sink_after: Some(0),
        ..Faults::default()
    }
}

pub(super) fn tree_join(
    port: Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
    faults: Faults,
) -> Observation {
    let (elements, mut symbols) = wide_tree(1, false);
    let left = table_of(&[CellKind::Node], &[&[node(1)], &[node(2)]]);
    let join = operation(
        &[0],
        &[1],
        &[(Side::Right, 0), (Side::Right, 1)],
        Vec::new(),
        &[],
    );
    invoke(
        &join,
        [Rows::Computed(&left), tree_rows(&elements, TreeForm::Edges)],
        &mut symbols,
        b"abcd",
        true,
        port,
        faults,
    )
    .unwrap()
}

pub(super) fn ordered(
    port: Arc<OrderedPort>,
    faults: Faults,
) -> Result<Observation, OperatingError> {
    let (request, _) = super::super::super::route_controls::fixtures::ordered_strategy_fixture();
    execute(
        request,
        ordered_sources(),
        &mut Symbols::default(),
        b"",
        true,
        port,
        faults,
    )
}

/// The strategy a row runs and what its meaning and its sink do.
#[derive(Clone, Copy, Debug)]
pub(super) struct Run {
    /// The native tree Join, or the ordered-existence strategy.
    pub(super) tree: bool,
    /// Whether the meaning refuses for a missing column.
    pub(super) meaning_refuses: bool,
    /// Whether the sink refuses the first row.
    pub(super) sink_refuses: bool,
}

/// A completed run: whether its meaning ran, its rows, the rows delivered and what the graph owner held once the
/// file finished.
#[derive(Debug, PartialEq)]
pub(super) struct Delivered {
    pub(super) meaning_ran: bool,
    pub(super) rows: usize,
    pub(super) delivered: usize,
    pub(super) held: Held,
}

/// A refused run: whether its meaning ran, the refusal, the rows delivered and what the graph owner held once the
/// file finished.
#[derive(Debug, PartialEq)]
pub(super) struct Refused {
    pub(super) meaning_ran: bool,
    pub(super) error: OperatingError,
    pub(super) delivered: usize,
    pub(super) held: Held,
}

/// Runs the strategy; `execute` finishes the owner and observes what it still held.
pub(super) fn exercise(run: Run) -> Result<Delivered, Refused> {
    let faults = if run.sink_refuses {
        cancelled()
    } else {
        Faults::default()
    };
    let (actual, calls) = if run.tree {
        let witness = Arc::new(Witness {
            refuse: run.meaning_refuses,
            ..Witness::default()
        });
        let actual = tree_join(witness.clone(), faults);
        (actual, witness.calls.load(Ordering::Relaxed))
    } else {
        let port = Arc::new(OrderedPort {
            refuse: run.meaning_refuses,
            ..OrderedPort::default()
        });
        let actual = ordered(port.clone(), faults).unwrap();
        (actual, port.calls.load(Ordering::Relaxed))
    };
    match actual.result {
        Ok(table) => Ok(Delivered {
            meaning_ran: calls > 0,
            rows: table.count.0,
            delivered: actual.prefix.len(),
            held: actual.held,
        }),
        Err(error) => Err(Refused {
            meaning_ran: calls > 0,
            error,
            delivered: actual.prefix.len(),
            held: actual.held,
        }),
    }
}
