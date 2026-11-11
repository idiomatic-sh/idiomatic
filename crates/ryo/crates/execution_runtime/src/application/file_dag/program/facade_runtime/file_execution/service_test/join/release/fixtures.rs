//! Strategy inputs and the one real semantic dependency whose answers are refused.
use super::super::super::super::*;
use super::super::capability::fixtures::REFUSAL;
use super::super::*;
use crate::outbound::operators::interface::OperatorsPort as _;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Flag, Integer, Offset, Relation, Span, Value as Cell},
};
use ryo_values::domain::rows::types::{RowCount, Table};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Counts the ordered-existence meaning and refuses it on request; every other call is real.
#[derive(Debug, Default)]
pub(super) struct OrderedPort {
    pub(super) calls: AtomicUsize,
    pub(super) refuse: bool,
}
impl crate::outbound::operators::interface::OperatorsPort for OrderedPort {
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
/// The error every refusing port raises, as the owner reports it.
pub(super) fn refused() -> OperatingError {
    OperatingError::Table(ryo_values::domain::rows::errors::RowsError::NoSuchColumn {
        index: REFUSAL,
    })
}
/// Three source rows whose numbers are 1, 3 and 2, with the strategy fixture's own schema.
pub(super) fn ordered_sources() -> [Provider<'static>; 2] {
    let (_, schema) = super::super::super::route_controls::test::ordered_strategy_fixture();
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
        super::super::fixtures::provider::stored(Relation::SourceLocation, source),
        super::super::fixtures::provider::stored(Relation::Function, unused),
    ]
}
