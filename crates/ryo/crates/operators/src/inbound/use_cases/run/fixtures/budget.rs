//! The run budget's workload: one file whose Defines rows all hold one name, joined with themselves on it, so the join writes
//! the square of the rows it reads.
use super::{TableInputs, TextResources, emitted};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{defines_rows, over_defines, over_nodes};
use crate::domain::physical_plan::types::PlanNodeId;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_relations::domain::relations::types::{Offset, Relation, Span};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, JoinColumn, KeyFan, Side, StepId,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::Table;

/// Runs the self join of `rows` Defines rows of one name, on the name, in one file over `buffers`: the join is node 1.
///
/// # Errors
///
/// The run's fault: `OverBudget` naming node 1 once the join writes past the run budget.
pub fn one_name_product(
    rows: u32,
    buffers: &mut FileBuffers,
) -> Result<Vec<Table>, PortableOperatorFault> {
    let mut resources = TextResources::new(&["k"]);
    let name = resources.symbol("k");
    let defines: Vec<_> = (0..rows)
        .map(|start| {
            (
                name.clone(),
                Span {
                    start: Offset(start),
                    end: Offset(start),
                },
            )
        })
        .collect();
    let table = defines_rows(&defines);
    let inputs = TableInputs::new(&[(Relation::Defines, &table)]);
    let column = |side, at| JoinColumn {
        side,
        column: ColumnIndex(at),
    };
    let steps = [
        over_defines(Operation::Select {
            from: StepInput::Relation(Relation::Defines),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        over_nodes(
            Operation::Join {
                left: StepInput::Step(StepId(0)),
                right: StepInput::Step(StepId(0)),
                left_on: vec![ColumnIndex(3)],
                right_on: vec![ColumnIndex(3)],
                output: vec![column(Side::Left, 0), column(Side::Right, 0)],
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
                fan: KeyFan::Shared,
            },
            &[0, 0],
        ),
    ];
    emitted(&steps, &[PlanNodeId(1)], &inputs, &mut resources, buffers)
}
