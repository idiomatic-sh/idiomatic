//! The facts a file run reads, held by the facts port's fake.
use ryo_execution::inbound::use_cases::execution::interface::FactsPort as _;
use ryo_execution::inbound::use_cases::execution::interface::FactsWorker;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use ryo_relations::domain::relations::fixtures::{kinds, to_bits};
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Flag, Integer, NodeId, Offset, Relation, Span, Value,
};
use ryo_relations::domain::terms::types::Symbol;

/// A worker of `facts` holding one row of `function` per id, in the file `file`.
pub(crate) fn function_rows(facts: &FactsFake, file: FileId, ids: &[u32]) -> Box<dyn FactsWorker> {
    let mut worker = facts.worker().expect("a fake worker");
    {
        let columns = worker.columns_mut().expect("the worker's columns");
        for id in ids {
            for (column, kind) in columns.function.iter_mut().zip(kinds(Relation::Function)) {
                let value = match kind {
                    CellKind::Node => Value::Node(NodeId(u64::from(*id))),
                    CellKind::Symbol => Value::Symbol(Symbol(0)),
                    CellKind::File => Value::File(file),
                    CellKind::Span => Value::Span(Span {
                        start: Offset(0),
                        end: Offset(0),
                    }),
                    CellKind::Number => Value::Number(Integer(0)),
                    CellKind::Flag => Value::Flag(Flag::Lowered),
                };
                column.0.push(to_bits(value).0);
            }
        }
    }
    worker
}
