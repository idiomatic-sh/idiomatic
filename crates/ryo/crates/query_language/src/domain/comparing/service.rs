//! The `comparing` service: the sides of a comparison are resolved, and the test is checked against
//! the types of its sides.

use ryo_relations::domain::relations::constants::{LOWERED, RAISED};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::terms::service::{owner_of, words_of};
use ryo_relations::domain::terms::types::Vocabulary;

use super::constants::{ENDS_WITH, MATCHES, STARTS_WITH, WORD_SEPARATOR, arity, function_name};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{
    Addressable, Builder, Compatibility, OriginCount, PlannedColumn, ResolvedOperand, SearchMatch,
    Typed,
};
use crate::domain::resolving::service::{column_of, find_parameter, parameter_of, position};
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plan_parts::types::ComparisonOriginId;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::program::types as syntax;
use ryo_values::domain::query_bindings::types::ParameterType;
use ryo_values::domain::query_syntax::types::{
    Alias, ColumnReference, Filter, WrittenApplied, WrittenFunction, WrittenOperand, WrittenTest,
};
use ryo_values::domain::query_tokens::types::QuerySpan;
use ryo_values::domain::query_tokens::types::QuerySpelling;

/// Whether a side is text: a name or a file column, text the query wrote, a text parameter or a function's text.
fn is_text(side: Typed) -> SearchMatch {
    if matches!(
        side,
        Typed::Column(CellKind::Symbol | CellKind::File)
            | Typed::Text
            | Typed::Parameter(ParameterType::Text)
    ) {
        SearchMatch::Matches
    } else {
        SearchMatch::Differs
    }
}

/// Whether `flag` is a flag column and `word` the text of a word a flag is shown as, `true` or `false`: `import = "false"`. Any other
/// text is refused, as a word of another kind is.
fn flag_word(flag: Typed, word: &parts::Operand) -> SearchMatch {
    if flag == Typed::Column(CellKind::Flag)
        && matches!(word, parts::Operand::Literal(parts::QueryLiteral::Text(text)) if *text.0 == *RAISED || *text.0 == *LOWERED)
    {
        SearchMatch::Matches
    } else {
        SearchMatch::Differs
    }
}

/// Whether the two sides can be compared by `test`.
pub(crate) fn compatible(test: parts::Test, left: Typed, right: Typed) -> Compatibility {
    let numbery = |side: Typed| {
        matches!(
            side,
            Typed::Column(CellKind::Number)
                | Typed::Number
                | Typed::Parameter(ParameterType::Number)
        )
    };
    let texty = |side: Typed| is_text(side) == SearchMatch::Matches;
    let comparable = match test {
        parts::Test::Equal | parts::Test::NotEqual => {
            (numbery(left) && numbery(right))
                || (texty(left) && texty(right))
                || (left == right && matches!(left, Typed::Column(_)))
                || matches!(
                    (left, right),
                    (
                        Typed::Column(CellKind::Flag),
                        Typed::Parameter(ParameterType::Flag)
                    )
                )
        }
        parts::Test::LessThan
        | parts::Test::LessThanOrEqual
        | parts::Test::GreaterThan
        | parts::Test::GreaterThanOrEqual => {
            (numbery(left) && numbery(right)) || (texty(left) && texty(right))
        }
        parts::Test::NumericGreaterThanOrEqual | parts::Test::IntegerSuccessor => {
            (texty(left) || numbery(left)) && (texty(right) || numbery(right))
        }
        parts::Test::NaturalLessThan
        | parts::Test::FloatTextGreaterThanOrEqual
        | parts::Test::StartsWith
        | parts::Test::Matches
        | parts::Test::EndsWith => texty(left) && texty(right),
        parts::Test::In | parts::Test::NotIn => {
            (numbery(left) && right == Typed::Parameter(ParameterType::Numbers))
                || (texty(left) && right == Typed::Parameter(ParameterType::Texts))
        }
        parts::Test::GlobAny | parts::Test::NotGlobAny => {
            texty(left) && (texty(right) || right == Typed::Parameter(ParameterType::Texts))
        }
    };
    if comparable {
        Compatibility::Comparable
    } else {
        Compatibility::Mismatched
    }
}

/// The test a filter's operator is.
pub(crate) fn test_of(test: WrittenTest) -> parts::Test {
    match test {
        WrittenTest::NaturalLessThan => parts::Test::NaturalLessThan,
        WrittenTest::Equal => parts::Test::Equal,
        WrittenTest::NotEqual => parts::Test::NotEqual,
        WrittenTest::LessThan => parts::Test::LessThan,
        WrittenTest::LessThanOrEqual => parts::Test::LessThanOrEqual,
        WrittenTest::GreaterThan => parts::Test::GreaterThan,
        WrittenTest::GreaterThanOrEqual => parts::Test::GreaterThanOrEqual,
        WrittenTest::In => parts::Test::In,
        WrittenTest::NotIn => parts::Test::NotIn,
        WrittenTest::GlobAny => parts::Test::GlobAny,
        WrittenTest::NotGlobAny => parts::Test::NotGlobAny,
        WrittenTest::FloatTextGreaterThanOrEqual => parts::Test::FloatTextGreaterThanOrEqual,
        WrittenTest::NumericGreaterThanOrEqual => parts::Test::NumericGreaterThanOrEqual,
        WrittenTest::IntegerSuccessor => parts::Test::IntegerSuccessor,
        WrittenTest::StartsWith => parts::Test::StartsWith,
        WrittenTest::Matches => parts::Test::Matches,
        WrittenTest::EndsWith => parts::Test::EndsWith,
    }
}

/// The function a query applies, as the plan holds it.
fn function_of(function: WrittenFunction) -> parts::Function {
    match function {
        WrittenFunction::Concat => parts::Function::Concat,
        WrittenFunction::Pascal => parts::Function::Pascal,
        WrittenFunction::Camel => parts::Function::Camel,
        WrittenFunction::Snake => parts::Function::Snake,
        WrittenFunction::Kebab => parts::Function::Kebab,
        WrittenFunction::Lower => parts::Function::Lower,
    }
}

/// A function applied to its arguments: each argument text, as many as the function takes; the function's answer is text.
fn applied_of(
    builder: &Builder<'_>,
    applied: &WrittenApplied,
    columns: Addressable<'_>,
) -> Result<ResolvedOperand, LoweringError> {
    let name = function_name(applied.function);
    let (takes, fits) = arity(applied.function, applied.arguments.len());
    if !fits {
        return Err(LoweringError::FunctionArity {
            function: name.to_owned(),
            takes,
            span: applied.span,
        });
    }
    let mut arguments = Vec::with_capacity(applied.arguments.len());
    for (place, argument) in applied.arguments.iter().enumerate() {
        let ResolvedOperand { operand, typed } = operand_of(builder, argument, columns)?;
        if is_text(typed) == SearchMatch::Differs {
            return Err(LoweringError::NotText {
                function: name.to_owned(),
                argument: place + 1,
                span: applied.span,
            });
        }
        arguments.push(operand);
    }
    Ok(ResolvedOperand {
        operand: parts::Operand::Function(parts::Applied {
            function: function_of(applied.function),
            arguments,
        }),
        typed: Typed::Text,
    })
}

/// One side of a comparison, with what the query says it is.
pub(crate) fn operand_of(
    builder: &Builder<'_>,
    operand: &WrittenOperand,
    columns: Addressable<'_>,
) -> Result<ResolvedOperand, LoweringError> {
    match operand {
        WrittenOperand::Column(reference) => {
            let index = column_index(builder, reference, columns)?;
            let kind = column_of(columns.fields, index).kind;
            Ok(ResolvedOperand {
                operand: parts::Operand::Column(index),
                typed: Typed::Column(kind),
            })
        }
        WrittenOperand::Parameter(name) => {
            let id = find_parameter(builder.parameters, |parameter| {
                if parameter.parameter == *name {
                    SearchMatch::Matches
                } else {
                    SearchMatch::Differs
                }
            })
            .ok_or_else(|| LoweringError::UnknownColumn {
                name: name.0.to_string(),
                span: builder.span,
            })?;
            Ok(ResolvedOperand {
                operand: parts::Operand::Parameter(id),
                typed: Typed::Parameter(*parameter_of(&builder.kinds, id)),
            })
        }
        WrittenOperand::Text(text) => Ok(ResolvedOperand {
            operand: parts::Operand::Literal(parts::QueryLiteral::Text(parts::LiteralText(
                text.0.clone(),
            ))),
            typed: Typed::Text,
        }),
        WrittenOperand::Number(number) => Ok(ResolvedOperand {
            operand: parts::Operand::Literal(parts::QueryLiteral::Number(
                ryo_relations::domain::relations::types::Integer(number.0),
            )),
            typed: Typed::Number,
        }),
        WrittenOperand::Function(applied) => applied_of(builder, applied, columns),
    }
}

/// Where the column `reference` names sits: after a join, `alias.column` is that side's column; otherwise the column of that name.
fn column_index(
    builder: &Builder<'_>,
    reference: &ColumnReference,
    columns: Addressable<'_>,
) -> Result<parts::ColumnIndex, LoweringError> {
    if let Alias::Written(alias) = &reference.alias
        && let Some(side) = columns
            .sides
            .iter()
            .find(|side| side.alias == *alias && side.column == reference.column)
    {
        return Ok(side.index);
    }
    position(
        columns.fields,
        QuerySpelling(&reference.column.0),
        builder.span,
    )
}

/// The comparison a filter makes over `columns`: its two operands resolved and checked to be of kinds that compare.
pub(crate) fn comparison_of(
    builder: &mut Builder<'_>,
    filter: &Filter,
    columns: Addressable<'_>,
) -> Result<parts::PlanComparison, LoweringError> {
    let ResolvedOperand {
        operand: left,
        typed: left_type,
    } = operand_of(builder, &filter.left, columns)?;
    let ResolvedOperand {
        operand: right,
        typed: right_type,
    } = operand_of(builder, &filter.right, columns)?;
    let test = test_of(filter.test);
    let affix = match test {
        parts::Test::NaturalLessThan => Some(super::constants::NATURAL_LESS_THAN),
        parts::Test::StartsWith => Some(STARTS_WITH),
        parts::Test::Matches => Some(MATCHES),
        parts::Test::EndsWith => Some(ENDS_WITH),
        parts::Test::Equal
        | parts::Test::NotEqual
        | parts::Test::LessThan
        | parts::Test::LessThanOrEqual
        | parts::Test::GreaterThan
        | parts::Test::GreaterThanOrEqual
        | parts::Test::FloatTextGreaterThanOrEqual
        | parts::Test::NumericGreaterThanOrEqual
        | parts::Test::IntegerSuccessor
        | parts::Test::In
        | parts::Test::NotIn
        | parts::Test::GlobAny
        | parts::Test::NotGlobAny => None,
    };
    if let Some(name) = affix
        && let Some(place) = [left_type, right_type]
            .iter()
            .position(|side| is_text(*side) == SearchMatch::Differs)
    {
        return Err(LoweringError::NotText {
            function: name.to_owned(),
            argument: place + 1,
            span: filter.span,
        });
    }
    let worded = matches!(test, parts::Test::Equal | parts::Test::NotEqual)
        && (flag_word(left_type, &right) == SearchMatch::Matches
            || flag_word(right_type, &left) == SearchMatch::Matches);
    if compatible(test, left_type, right_type) == Compatibility::Mismatched && !worded {
        return Err(LoweringError::KindMismatch { span: filter.span });
    }
    check_words(filter, &left, &right, columns.fields)?;
    // Literal patterns are checked before optimization or execution, including empty inputs, by parsing them with the syntax the
    // matcher compiles: the one compile of a literal pattern is the executor's prepared slot, on first use.
    // Dynamic patterns remain checked by the executor when their values are available.
    if test == parts::Test::Matches
        && let parts::Operand::Literal(parts::QueryLiteral::Text(pattern)) = &right
    {
        let _parsed = regex_automata::util::syntax::parse(&pattern.0).map_err(|error| {
            LoweringError::InvalidRegex {
                reason: error.to_string(),
                span: filter.span,
            }
        })?;
    }
    let origin = if let Some(mut origin) = builder.origin.clone() {
        origin.span = filter.span;
        let id = comparison_origin_id(OriginCount(builder.comparison_origins.len()), filter.span)?;
        builder.comparison_origins.push(origin);
        parts::ComparisonOrigin::Recorded(id)
    } else {
        parts::ComparisonOrigin::Unrecorded
    };
    Ok(parts::PlanComparison {
        test,
        left,
        right,
        origin,
    })
}

/// The id the next comparison origin is interned under, when `held` are interned already.
pub(crate) fn comparison_origin_id(
    held: OriginCount,
    span: QuerySpan,
) -> Result<ComparisonOriginId, LoweringError> {
    u32::try_from(held.0)
        .map(ComparisonOriginId)
        .map_err(|_overflow| LoweringError::TooManyComparisonOrigins { span })
}

/// That a word compared with a column of a closed set of words is one of them: `kind = "headline"` is refused when the query is compiled,
/// naming the words there are, never a comparison that matches nothing.
fn check_words(
    filter: &Filter,
    left: &parts::Operand,
    right: &parts::Operand,
    fields: &[PlannedColumn],
) -> Result<(), LoweringError> {
    let (parts::Test::Equal | parts::Test::NotEqual) = test_of(filter.test) else {
        return Ok(());
    };
    let (column, word) = match (left, right) {
        (
            parts::Operand::Column(column),
            parts::Operand::Literal(parts::QueryLiteral::Text(text)),
        )
        | (
            parts::Operand::Literal(parts::QueryLiteral::Text(text)),
            parts::Operand::Column(column),
        ) => (column_of(fields, *column), text),
        _ => return Ok(()),
    };
    let Vocabulary::Closed(block) = column.words else {
        return Ok(());
    };
    let words = words_of(block);
    if words.iter().any(|each| each.0 == &*word.0) {
        return Ok(());
    }
    Err(LoweringError::UnknownWord {
        word: word.0.to_string().into(),
        column: column.name.0.to_string().into(),
        owner: owner_of(block).0.into(),
        words: words
            .iter()
            .map(|each| each.0)
            .collect::<Vec<_>>()
            .join(WORD_SEPARATOR)
            .into(),
        span: filter.span,
    })
}
