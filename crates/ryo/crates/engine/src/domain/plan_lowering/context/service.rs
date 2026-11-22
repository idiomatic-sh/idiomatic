//! The `context` service: the steps a context table and a rule observation are made of.

use crate::domain::plan_lowering::types::ContextTable;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::finish_plans::arguments::literals::types::{
    ArgumentValue as StepArgument, StepArgumentName, UnlistedArgument,
};
use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::constants::{ARG_COLUMNS, ARG_FROM, ARG_LEFT, ARG_RIGHT};
use ryo_values::domain::finish_plans::expressions::types::{ColumnRead, PlanConstant, PlanExpression, SourceAlias};
use ryo_values::domain::finish_plans::types::{
    CountMark, FinishStep, ProbeMark, RowCondition, SelectItem, SelectSource, SelectStep, SelectedColumns,
};
use ryo_values::domain::plan_parts::types::LiteralText;

/// The arguments of the `rows` step a context table is: its rows, then its columns.
pub(crate) fn table_arguments(table: &ContextTable) -> Vec<UnlistedArgument> {
    let text = |value: &str| PlanConstant::Text(LiteralText(value.into()));
    let rows = table.rows.iter().map(|row| PlanConstant::List(row.iter().map(|value| text(value)).collect())).collect();
    let columns = table.columns.iter().map(|column| text(&column.0)).collect();
    vec![argument(ARG_FROM, StepArgument::Constant(PlanConstant::List(rows))), argument(ARG_COLUMNS, StepArgument::Constant(PlanConstant::List(columns)))]
}

/// The select keeping an observation's columns of one producer's output.
pub(crate) fn producer_select(output: StepNumber, columns: &[ColumnName]) -> FinishStep {
    let items = columns
        .iter()
        .map(|column| SelectItem {
            name: column.clone(),
            value: PlanExpression::Column(ColumnRead { column: column.clone(), alias: SourceAlias::Unwritten }),
        })
        .collect();
    FinishStep::Select(SelectStep {
        sources: vec![SelectSource { step: output, alias: SourceAlias::Unwritten }],
        columns: SelectedColumns::Items(items),
        condition: RowCondition::Every,
        probe: ProbeMark::Unmarked,
        count: CountMark::Unmarked,
    })
}

/// The arguments of the `union` of the producers united so far and the next.
pub(crate) fn union_arguments(left: StepNumber, right: StepNumber) -> Vec<UnlistedArgument> {
    vec![argument(ARG_LEFT, StepArgument::Step(left)), argument(ARG_RIGHT, StepArgument::Step(right))]
}

fn argument(name: &str, value: StepArgument) -> UnlistedArgument {
    UnlistedArgument { name: StepArgumentName(name.into()), value }
}
