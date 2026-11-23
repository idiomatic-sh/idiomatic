//! The `filters` service: conditions and constants, from the parser's form to the plan's.

use super::constants::{
    CAMEL, CONCAT, ENDS_WITH, FLOAT_TEXT_GTE, GLOB_ANY, INTEGER_SUCCESSOR, KEBAB, LOWER, MATCHES,
    NATURAL_LT, NUMERIC_GTE, PASCAL, SNAKE, STARTS_WITH, TRUTH_WORDS,
};
use super::types::Parameters;
use ryo_values::domain::finish_plans::expressions::types::{
    ColumnRead, Comparator, ConstantEntry, ConstantEntryKey, ExpressionPair, FunctionCall,
    FunctionName, PlanConstant, PlanExpression, PlanNumber, RowComparison, SourceAlias,
};
use ryo_values::domain::finish_plans::types::RowCondition;
use crate::domain::plan_lowering::errors::PlanLoweringError;
use crate::domain::plan_lowering::types::WrittenText;
use ryo_values::domain::plan_parts::types::{LiteralText, WholeNumber};
use ryo_values::domain::query_syntax::types::{
    Alias, ArgumentValue, ColumnReference, Condition, Filter, Joining, Select, WrittenApplied,
    WrittenFunction, WrittenOperand, WrittenTest,
};

/// A select's one condition: its `on` keys, each `left = right`, and its `where` conditions, each side a left-to-right
/// `and`; both when it has both.
///
/// # Errors
///
/// `UnknownParameter` for a `$` naming no parameter.
pub(crate) fn select_condition(
    select: &Select,
    parameters: &Parameters,
) -> Result<RowCondition, PlanLoweringError> {
    let keys = match &select.join {
        Joining::Alone => None,
        Joining::Joined(join) => folded(
            join.on.iter().map(|key| {
                PlanExpression::Compare(RowComparison {
                    comparator: Comparator::Equal,
                    left: Box::new(column(&key.left)),
                    right: Box::new(column(&key.right)),
                })
            }),
            PlanExpression::And,
        ),
    };
    let filters = conjunction(&select.filters, parameters)?;
    Ok(
        match folded(keys.into_iter().chain(filters), PlanExpression::And) {
            None => RowCondition::Every,
            Some(condition) => RowCondition::Tested(condition),
        },
    )
}

/// Expressions joined left to right by `join`: none for none.
fn folded(
    expressions: impl IntoIterator<Item = PlanExpression>,
    join: fn(ExpressionPair) -> PlanExpression,
) -> Option<PlanExpression> {
    expressions.into_iter().reduce(|left, right| {
        join(ExpressionPair {
            left: Box::new(left),
            right: Box::new(right),
        })
    })
}

/// Conditions that all hold, as one left-to-right `and`: none for none.
fn conjunction(
    conditions: &[Condition],
    parameters: &Parameters,
) -> Result<Option<PlanExpression>, PlanLoweringError> {
    let read = conditions
        .iter()
        .map(|condition| match condition {
            Condition::Test(filter) => test(filter, parameters),
            Condition::Not(negated) => conjunction(negated, parameters)?
                .map(|holds| PlanExpression::Not(Box::new(holds)))
                .ok_or_else(|| PlanLoweringError::NotGiven(WrittenText("a negated condition".into()))),
            Condition::Any(alternatives) => {
                let each = alternatives
                    .iter()
                    .map(|alternative| conjunction(alternative, parameters))
                    .collect::<Result<Vec<_>, _>>()?;
                folded(each.into_iter().flatten(), PlanExpression::Or).ok_or_else(|| {
                    PlanLoweringError::NotGiven(WrittenText("an alternative".into()))
                })
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(folded(read, PlanExpression::And))
}

/// One filter: a comparison, a membership, or a function of its two operands.
fn test(filter: &Filter, parameters: &Parameters) -> Result<PlanExpression, PlanLoweringError> {
    let left = operand(&filter.left, parameters)?;
    let right = operand(&filter.right, parameters)?;
    let compared = |comparator, left, right| {
        Ok(PlanExpression::Compare(RowComparison {
            comparator,
            left: Box::new(left),
            right: Box::new(right),
        }))
    };
    let pair = |left, right| ExpressionPair {
        left: Box::new(left),
        right: Box::new(right),
    };
    let called = |name: &str, left, right| {
        PlanExpression::Call(FunctionCall {
            function: FunctionName(name.into()),
            arguments: vec![left, right],
        })
    };
    match filter.test {
        WrittenTest::Equal => compared(Comparator::Equal, left, right),
        WrittenTest::NotEqual => compared(Comparator::NotEqual, left, right),
        WrittenTest::LessThan => compared(Comparator::Less, left, right),
        WrittenTest::LessThanOrEqual => compared(Comparator::LessOrEqual, left, right),
        WrittenTest::GreaterThan => compared(Comparator::Greater, left, right),
        WrittenTest::GreaterThanOrEqual => compared(Comparator::GreaterOrEqual, left, right),
        WrittenTest::In => Ok(PlanExpression::In(pair(left, right))),
        WrittenTest::NotIn => Ok(PlanExpression::NotIn(pair(left, right))),
        WrittenTest::GlobAny => Ok(called(GLOB_ANY, left, right)),
        WrittenTest::NotGlobAny => Ok(PlanExpression::Not(Box::new(called(GLOB_ANY, left, right)))),
        WrittenTest::FloatTextGreaterThanOrEqual => Ok(called(FLOAT_TEXT_GTE, left, right)),
        WrittenTest::StartsWith => Ok(called(STARTS_WITH, left, right)),
        WrittenTest::EndsWith => Ok(called(ENDS_WITH, left, right)),
        WrittenTest::Matches => Ok(called(MATCHES, left, right)),
        WrittenTest::NumericGreaterThanOrEqual => Ok(called(NUMERIC_GTE, left, right)),
        WrittenTest::IntegerSuccessor => Ok(called(INTEGER_SUCCESSOR, left, right)),
        WrittenTest::NaturalLessThan => Ok(called(NATURAL_LT, left, right)),
    }
}

/// One side of a filter: a column (`true` and `false` written bare are constants), a parameter's value, text, a number,
/// or a function of operands.
pub(crate) fn operand(
    written: &WrittenOperand,
    parameters: &Parameters,
) -> Result<PlanExpression, PlanLoweringError> {
    match written {
        WrittenOperand::Column(reference) => Ok(
            if reference.alias == Alias::Unwritten && TRUTH_WORDS.contains(&&*reference.column.0) {
                PlanExpression::Literal(PlanConstant::Text(LiteralText(
                    reference.column.0.clone(),
                )))
            } else {
                column(reference)
            },
        ),
        WrittenOperand::Parameter(name) => Ok(match parameter(&name.0, parameters)? {
            PlanConstant::Number(number) => PlanExpression::Number(number),
            other @ (PlanConstant::Text(_)
            | PlanConstant::List(_)
            | PlanConstant::Map(_)
            | PlanConstant::Absent) => PlanExpression::Literal(other),
        }),
        WrittenOperand::Text(text) => Ok(PlanExpression::Literal(PlanConstant::Text(
            LiteralText(text.0.clone()),
        ))),
        WrittenOperand::Number(number) => {
            Ok(PlanExpression::Number(PlanNumber::Whole(WholeNumber(number.0))))
        }
        WrittenOperand::Function(applied) => function(applied, parameters),
    }
}

/// A function applied to operands.
pub(crate) fn function(
    applied: &WrittenApplied,
    parameters: &Parameters,
) -> Result<PlanExpression, PlanLoweringError> {
    let name = match applied.function {
        WrittenFunction::Concat => CONCAT,
        WrittenFunction::Pascal => PASCAL,
        WrittenFunction::Camel => CAMEL,
        WrittenFunction::Snake => SNAKE,
        WrittenFunction::Kebab => KEBAB,
        WrittenFunction::Lower => LOWER,
    };
    Ok(PlanExpression::Call(FunctionCall {
        function: FunctionName(name.into()),
        arguments: applied
            .arguments
            .iter()
            .map(|argument| operand(argument, parameters))
            .collect::<Result<_, _>>()?,
    }))
}

/// A column, under the alias the query writes before it.
pub(crate) fn column(reference: &ColumnReference) -> PlanExpression {
    PlanExpression::Column(ColumnRead {
        column: reference.column.clone(),
        alias: alias(&reference.alias),
    })
}

/// The parser's alias as the plan's.
pub(crate) fn alias(written: &Alias) -> SourceAlias {
    match written {
        Alias::Unwritten => SourceAlias::Unwritten,
        Alias::Written(name) => SourceAlias::Written(name.clone()),
    }
}

/// The value a parameter was given.
fn parameter(name: &str, parameters: &Parameters) -> Result<PlanConstant, PlanLoweringError> {
    parameters
        .values
        .get(name)
        .cloned()
        .ok_or_else(|| PlanLoweringError::UnknownParameter(WrittenText(name.into())))
}

/// A constant argument's value: text, a number, a word, a list, a map (a key written twice keeps its first place and
/// its last value) or a parameter's value.
///
/// # Errors
///
/// `UnknownParameter` for a `$` naming no parameter.
pub(crate) fn constant(
    value: &ArgumentValue,
    parameters: &Parameters,
) -> Result<PlanConstant, PlanLoweringError> {
    match value {
        ArgumentValue::Text(text) => Ok(PlanConstant::Text(LiteralText(text.0.clone()))),
        ArgumentValue::Number(number) => Ok(PlanConstant::Number(PlanNumber::Whole(WholeNumber(
            number.0,
        )))),
        ArgumentValue::Word(word) => Ok(PlanConstant::Text(LiteralText(word.0.clone()))),
        ArgumentValue::Step(step) => Ok(PlanConstant::Text(LiteralText(step.0.0.clone()))),
        ArgumentValue::Parameter(name) => parameter(&name.0, parameters),
        ArgumentValue::List(items) => Ok(PlanConstant::List(
            items
                .iter()
                .map(|item| constant(item, parameters))
                .collect::<Result<_, _>>()?,
        )),
        ArgumentValue::Map(arguments) => {
            let mut entries: Vec<ConstantEntry> = Vec::new();
            for argument in arguments {
                let key = ConstantEntryKey(argument.name.0.clone());
                let value = constant(&argument.value, parameters)?;
                match entries.iter_mut().find(|entry| entry.key == key) {
                    Some(entry) => entry.value = value,
                    None => entries.push(ConstantEntry { key, value }),
                }
            }
            Ok(PlanConstant::Map(entries))
        }
    }
}
