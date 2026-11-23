//! The `joins` service: a `join` call's select.

use super::constants::{JOIN, LEFT_ALIAS, RIGHT_ALIAS};
use crate::domain::plan_lowering::errors::PlanLoweringError;
use crate::domain::plan_lowering::types::WrittenText;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::constants::{ARG_LEFT, ARG_LEFT_ON, ARG_OUTPUT, ARG_RIGHT, ARG_RIGHT_ON};
use ryo_values::domain::finish_plans::expressions::types::{
    ColumnRead, Comparator, ExpressionPair, PlanConstant, PlanExpression, RowComparison, SourceAlias,
};
use ryo_values::domain::finish_plans::types::{
    CountMark, FinishStep, ProbeMark, RowCondition, SelectItem, SelectSource, SelectStep, SelectedColumns,
};
use ryo_values::domain::query_tokens::types::AliasName;

/// The relations a `join` reads and its constant arguments, as the call gives them.
pub(crate) struct JoinCall<'c> {
    pub(crate) left: StepNumber,
    pub(crate) right: StepNumber,
    pub(crate) left_on: &'c PlanConstant,
    pub(crate) right_on: &'c PlanConstant,
    pub(crate) output: &'c PlanConstant,
}

/// The select a `join` is: the left source then the right, the key pairs' equalities in order (each conjoined to the
/// ones before it), the output's columns in the order written.
pub(crate) fn joined(call: &JoinCall<'_>) -> Result<FinishStep, PlanLoweringError> {
    let left_on = names(call.left_on, ARG_LEFT_ON)?;
    let right_on = names(call.right_on, ARG_RIGHT_ON)?;
    if left_on.len() != right_on.len() {
        return Err(refused(ARG_RIGHT_ON));
    }
    let mut condition: Option<PlanExpression> = None;
    for (left, right) in left_on.iter().zip(&right_on) {
        let equal = PlanExpression::Compare(RowComparison {
            comparator: Comparator::Equal,
            left: Box::new(read(left, LEFT_ALIAS)),
            right: Box::new(read(right, RIGHT_ALIAS)),
        });
        condition = Some(match condition {
            None => equal,
            Some(earlier) => PlanExpression::And(ExpressionPair { left: Box::new(earlier), right: Box::new(equal) }),
        });
    }
    let PlanConstant::Map(entries) = call.output else {
        return Err(refused(ARG_OUTPUT));
    };
    let mut items = Vec::with_capacity(entries.len());
    for entry in entries {
        let [PlanConstant::Text(side), PlanConstant::Text(column)] = constant_pair(&entry.value)? else {
            return Err(refused(ARG_OUTPUT));
        };
        let alias = match &*side.0 {
            ARG_LEFT => LEFT_ALIAS,
            ARG_RIGHT => RIGHT_ALIAS,
            _ => return Err(refused(ARG_OUTPUT)),
        };
        items.push(SelectItem { name: ColumnName(entry.key.0.clone()), value: read(&column.0, alias) });
    }
    Ok(FinishStep::Select(SelectStep {
        sources: vec![source(call.left, LEFT_ALIAS), source(call.right, RIGHT_ALIAS)],
        columns: SelectedColumns::Items(items),
        condition: condition.map_or(RowCondition::Every, RowCondition::Tested),
        probe: ProbeMark::Unmarked,
        count: CountMark::Unmarked,
    }))
}

/// The two values of an output entry (`[side, column]`).
fn constant_pair(value: &PlanConstant) -> Result<&[PlanConstant; 2], PlanLoweringError> {
    match value {
        PlanConstant::List(items) => <&[PlanConstant; 2]>::try_from(items.as_slice()).map_err(|_| refused(ARG_OUTPUT)),
        PlanConstant::Text(_) | PlanConstant::Number(_) | PlanConstant::Map(_) | PlanConstant::Absent => {
            Err(refused(ARG_OUTPUT))
        }
    }
}

/// The column names of a key list.
fn names<'c>(value: &'c PlanConstant, argument: &str) -> Result<Vec<&'c str>, PlanLoweringError> {
    let PlanConstant::List(items) = value else {
        return Err(refused(argument));
    };
    items
        .iter()
        .map(|item| match item {
            PlanConstant::Text(text) => Ok(&*text.0),
            PlanConstant::Number(_) | PlanConstant::List(_) | PlanConstant::Map(_) | PlanConstant::Absent => {
                Err(refused(argument))
            }
        })
        .collect()
}

/// A column read under an alias.
fn read(column: &str, alias: &str) -> PlanExpression {
    PlanExpression::Column(ColumnRead {
        column: ColumnName(column.into()),
        alias: SourceAlias::Written(AliasName(alias.into())),
    })
}

/// A source read under an alias.
fn source(step: StepNumber, alias: &str) -> SelectSource {
    SelectSource { step, alias: SourceAlias::Written(AliasName(alias.into())) }
}

/// A `join` refused for one of its arguments.
fn refused(argument: &str) -> PlanLoweringError {
    PlanLoweringError::OperatorArgument { operator: WrittenText(JOIN.into()), argument: WrittenText(argument.into()) }
}
