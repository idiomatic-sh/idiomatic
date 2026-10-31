// ignore: idiomatic.arch/file-length - the winnow adapter is the RyoQL grammar: one small parser fn for each thing the language has, from a word to the whole program (30 fns, none long); splitting them across files would scatter one grammar.
//! The syntax port, implemented with winnow: RyoQL text is read by small parsers, one for each
//! thing the language has, from the smallest (a word, a string) up to the whole program.

use winnow::ascii::{digit1, multispace0, multispace1};
use winnow::combinator::{alt, cut_err, delimited, eof, opt, preceded, separated, terminated};
use winnow::prelude::*;
use winnow::stream::LocatingSlice;
use winnow::token::{take_till, take_while};

use super::constants::{GRAMMAR, Keyword, Mark, expectation, keyword_text, mark_text};
use crate::outbound::syntax::errors::SyntaxError;
use crate::outbound::syntax::interface::SyntaxPort;
use ryo_language::domain::relations::types::Integer;
use ryo_values::domain::descriptions::types::{
    Example, Grammar, Production, ProductionForm, ProductionName,
};
use ryo_values::domain::program::types::{
    Alias, AliasName, Argument, ArgumentName, ArgumentValue, BareWord, ByteOffset, Call,
    ColumnName, ColumnReference, Columns, Condition, Expression, Filter, Join, JoinKey, Joining,
    LetStep, OperationName, ParameterDeclaration, Program, QuerySource, QuerySpan, QueryView,
    Renaming, Select, SelectColumn, SourceReference, StepName, TypeName, WordText, WrittenApplied,
    WrittenEmit, WrittenFunction, WrittenLiteral, WrittenName, WrittenOperand, WrittenParameter,
    WrittenRelation, WrittenTest,
};

/// The winnow syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxWinnowAdapter;

/// What the parsers read: the text, which also knows where each parser is in it.
type Input<'i> = LocatingSlice<&'i str>;

/// One line of a program, before the lines are put together.
pub(super) enum ProgramLine {
    /// A `param` line: a parameter the program is given.
    Parameter(ParameterDeclaration),
    /// A `let` line: a step that makes rows.
    Step(LetStep),
    /// The `emit` line: the step whose rows are the findings.
    Emit(WrittenEmit),
}

impl SyntaxPort for SyntaxWinnowAdapter {
    fn read(&self, text: QueryView<'_>) -> Result<Program, SyntaxError> {
        program.parse(LocatingSlice::new(text.0)).map_err(|error| {
            // What winnow said, or what the language expects when it said nothing.
            let said = error.inner().to_string();
            SyntaxError::Unreadable {
                span: QuerySpan {
                    start: ByteOffset(error.offset()),
                    end: ByteOffset(error.offset()),
                },
                message: if said.is_empty() {
                    expectation().to_owned()
                } else {
                    said
                },
            }
        })
    }

    fn grammar(&self) -> Grammar {
        Grammar(
            GRAMMAR
                .iter()
                .map(|line| Production {
                    name: ProductionName(line.name.into()),
                    form: ProductionForm(line.form.into()),
                    example: Example(line.example.into()),
                })
                .collect(),
        )
    }
}

/// The whole text: lines until the end, put together as parameters, steps and the one `emit`.
pub(super) fn program(input: &mut Input<'_>) -> ModalResult<Program> {
    let mut parameters = Vec::new();
    let mut steps = Vec::new();
    let mut emit = None;
    loop {
        multispace0.void().parse_next(input)?;
        if opt(eof).parse_next(input)?.is_some() {
            break;
        }
        match line.parse_next(input)? {
            ProgramLine::Parameter(parameter) => parameters.push(parameter),
            ProgramLine::Step(step) => steps.push(step),
            ProgramLine::Emit(found) => {
                if emit.replace(found).is_some() {
                    return Err(winnow::error::ErrMode::Cut(
                        winnow::error::ContextError::new(),
                    ));
                }
            }
        }
    }
    match emit {
        Some(emit) => Ok(Program {
            parameters,
            steps,
            emit,
        }),
        None => Err(winnow::error::ErrMode::Cut(
            winnow::error::ContextError::new(),
        )),
    }
}

/// One line of a program: a parameter declaration, a `let` step or an `emit`.
pub(super) fn line(input: &mut Input<'_>) -> ModalResult<ProgramLine> {
    alt((
        parameter_line.map(ProgramLine::Parameter),
        step_line.map(ProgramLine::Step),
        emit_line.map(ProgramLine::Emit),
    ))
    .parse_next(input)
}

/// `param max-lines: integer;`.
pub(super) fn parameter_line(input: &mut Input<'_>) -> ModalResult<ParameterDeclaration> {
    let ((name, kind), span) = delimited(
        (keyword(Keyword::Param), multispace1),
        (
            terminated(word, (multispace0, ':', multispace0)),
            take_while(1.., ('a'..='z', 'A'..='Z', '0'..='9', '<', '>', '-', '_')),
        ),
        (multispace0, ';'),
    )
    .with_span()
    .parse_next(input)?;
    Ok(ParameterDeclaration {
        name: WrittenParameter(Box::from(name.0)),
        kind: TypeName(Box::from(kind)),
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// `let name = expression;`.
pub(super) fn step_line(input: &mut Input<'_>) -> ModalResult<LetStep> {
    let ((name, expression), span) = delimited(
        (keyword(Keyword::Let), multispace1),
        (
            terminated(word, (multispace0, '=', multispace0)),
            expression,
        ),
        (multispace0, ';'),
    )
    .with_span()
    .parse_next(input)?;
    Ok(LetStep {
        name: StepName(Box::from(name.0)),
        expression,
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// `emit step subject column key(columns);`.
pub(super) fn emit_line(input: &mut Input<'_>) -> ModalResult<WrittenEmit> {
    let ((step, subject, key), span) = delimited(
        (keyword(Keyword::Emit), multispace1),
        (
            terminated(word, (multispace1, keyword(Keyword::Subject), multispace1)),
            terminated(
                name_or_string.map(|name| ColumnName(name.0)),
                (multispace1, keyword(Keyword::Key), multispace0),
            ),
            delimited(
                ('(', multispace0),
                separated(
                    1..,
                    name_or_string.map(|name| ColumnName(name.0)),
                    (multispace0, ',', multispace0),
                ),
                (multispace0, ')'),
            ),
        ),
        (multispace0, ';'),
    )
    .with_span()
    .parse_next(input)?;
    let key: Vec<ColumnName> = key;
    Ok(WrittenEmit {
        step: StepName(Box::from(step.0)),
        subject,
        key,
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// The right side of a `let`: a `select` statement or an operation's call.
pub(super) fn expression(input: &mut Input<'_>) -> ModalResult<Expression> {
    alt((select.map(Expression::Select), call.map(Expression::Call))).parse_next(input)
}

/// `select columns from source join source on a = b where filters`.
pub(super) fn select(input: &mut Input<'_>) -> ModalResult<Select> {
    (keyword(Keyword::Select), multispace1)
        .void()
        .parse_next(input)?;
    let columns = alt((
        '*'.map(|_| Columns::Every),
        separated(1.., select_column, (multispace0, ',', multispace0)).map(Columns::Listed),
    ))
    .parse_next(input)?;
    (multispace1, keyword(Keyword::From), multispace1)
        .void()
        .parse_next(input)?;
    let from = source.parse_next(input)?;
    let join = opt(join)
        .parse_next(input)?
        .map_or(Joining::Alone, Joining::Joined);
    let filters = opt(filters).parse_next(input)?.unwrap_or_default();
    Ok(Select {
        columns,
        from,
        join,
        filters,
    })
}

/// One column a `select` lists, with the name it is renamed to when it has one.
pub(super) fn select_column(input: &mut Input<'_>) -> ModalResult<SelectColumn> {
    let column = column_reference.parse_next(input)?;
    let rename = opt(preceded(
        (multispace1, keyword(Keyword::As), multispace1),
        name_or_string.map(|name| ColumnName(name.0)),
    ))
    .parse_next(input)?
    .map_or(Renaming::Unchanged, Renaming::Renamed);
    Ok(SelectColumn { column, rename })
}

/// `"relation"` or `step`, then an optional `as alias`.
pub(super) fn source(input: &mut Input<'_>) -> ModalResult<QuerySource> {
    let ((reference, alias), span) = (
        alt((
            string.map(|name| SourceReference::Relation(WrittenRelation(name.0))),
            word.map(|name| SourceReference::Step(StepName(Box::from(name.0)))),
        )),
        opt(preceded(
            (multispace1, keyword(Keyword::As), multispace1),
            word,
        )),
    )
        .with_span()
        .parse_next(input)?;
    Ok(QuerySource {
        reference,
        alias: alias.map_or(Alias::Unwritten, |alias| {
            Alias::Written(AliasName(Box::from(alias.0)))
        }),
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// ` join source on a = b and c = d`.
pub(super) fn join(input: &mut Input<'_>) -> ModalResult<Join> {
    (multispace1, keyword(Keyword::Join), multispace1)
        .void()
        .parse_next(input)?;
    let source = source.parse_next(input)?;
    (multispace1, keyword(Keyword::On), multispace1)
        .void()
        .parse_next(input)?;
    let on = separated(
        1..,
        join_key,
        (multispace1, keyword(Keyword::And), multispace1),
    )
    .parse_next(input)?;
    Ok(Join { source, on })
}

/// `a = b`.
pub(super) fn join_key(input: &mut Input<'_>) -> ModalResult<JoinKey> {
    let left = column_reference.parse_next(input)?;
    (multispace0, '=', multispace0).void().parse_next(input)?;
    let right = column_reference.parse_next(input)?;
    Ok(JoinKey { left, right })
}

/// ` where condition`: the conditions, all of which must hold.
pub(super) fn filters(input: &mut Input<'_>) -> ModalResult<Vec<Condition>> {
    (multispace1, keyword(Keyword::Where), multispace1)
        .void()
        .parse_next(input)?;
    condition.map(conditions_of).parse_next(input)
}

/// `a or b and c`: the alternatives `or` separates, each a conjunction, so `and` binds tighter than `or`.
pub(super) fn condition(input: &mut Input<'_>) -> ModalResult<Vec<Vec<Condition>>> {
    separated(
        1..,
        conjunction,
        (multispace1, keyword(Keyword::Or), multispace1),
    )
    .parse_next(input)
}

/// `a and b`: conditions that all hold; a group in parentheses with no `or` is read as its conditions.
pub(super) fn conjunction(input: &mut Input<'_>) -> ModalResult<Vec<Condition>> {
    let factors: Vec<Vec<Condition>> = separated(
        1..,
        factor,
        (multispace1, keyword(Keyword::And), multispace1),
    )
    .parse_next(input)?;
    Ok(factors.into_iter().flatten().collect())
}

/// `(a or b)`, a condition in parentheses, or one filter.
pub(super) fn factor(input: &mut Input<'_>) -> ModalResult<Vec<Condition>> {
    alt((
        delimited(('(', multispace0), condition, (multispace0, ')')).map(conditions_of),
        filter.map(|test| vec![Condition::Test(test)]),
    ))
    .parse_next(input)
}

/// The conditions the alternatives of an `or` make: those of the one alternative, or one condition that any of them holds.
fn conditions_of(mut alternatives: Vec<Vec<Condition>>) -> Vec<Condition> {
    if alternatives.len() == 1 {
        alternatives.pop().unwrap_or_default()
    } else {
        vec![Condition::Any(alternatives)]
    }
}

/// One condition of a `where`: a glob, a comparison of texts or numbers, or a test of two operands.
pub(super) fn filter(input: &mut Input<'_>) -> ModalResult<Filter> {
    alt((
        glob_filter,
        float_text_filter,
        affix_filter,
        comparison_filter,
    ))
    .parse_next(input)
}

/// `starts_with(text, prefix)` and `ends_with(text, suffix)`.
pub(super) fn affix_filter(input: &mut Input<'_>) -> ModalResult<Filter> {
    let ((test, left, right), span) = (
        terminated(
            alt((
                keyword(Keyword::StartsWith).value(WrittenTest::StartsWith),
                keyword(Keyword::Matches).value(WrittenTest::Matches),
                keyword(Keyword::EndsWith).value(WrittenTest::EndsWith),
            )),
            (multispace0, '(', multispace0),
        ),
        terminated(operand, (multispace0, ',', multispace0)),
        terminated(operand, (multispace0, ')')),
    )
        .with_span()
        .parse_next(input)?;
    Ok(Filter {
        left,
        test,
        right,
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// `float_text_gte(left, right)`.
pub(super) fn float_text_filter(input: &mut Input<'_>) -> ModalResult<Filter> {
    let ((left, right), span) = (
        preceded(
            (
                keyword(Keyword::FloatTextGte),
                multispace0,
                '(',
                multispace0,
            ),
            terminated(operand, (multispace0, ',', multispace0)),
        ),
        terminated(operand, (multispace0, ')')),
    )
        .with_span()
        .parse_next(input)?;
    Ok(Filter {
        left,
        test: WrittenTest::FloatTextGreaterThanOrEqual,
        right,
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// `glob_any(left, right)` and `not glob_any(left, right)`.
pub(super) fn glob_filter(input: &mut Input<'_>) -> ModalResult<Filter> {
    let ((negated, left, right), span) = (
        opt((keyword(Keyword::Not), multispace1)).map(|not| not.is_some()),
        preceded(
            (keyword(Keyword::GlobAny), multispace0, '(', multispace0),
            terminated(operand, (multispace0, ',', multispace0)),
        ),
        terminated(operand, (multispace0, ')')),
    )
        .with_span()
        .parse_next(input)?;
    Ok(Filter {
        left,
        test: if negated {
            WrittenTest::NotGlobAny
        } else {
            WrittenTest::GlobAny
        },
        right,
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// `left = right`, `left in right`, and the rest of the comparisons.
pub(super) fn comparison_filter(input: &mut Input<'_>) -> ModalResult<Filter> {
    let ((left, test, right), span) = (
        terminated(operand, multispace0),
        terminated(test, multispace0),
        // Once a test is read, only its right side can follow: a failure here is reported where the side begins, not at the start of
        // the statement.
        cut_err(operand),
    )
        .with_span()
        .parse_next(input)?;
    Ok(Filter {
        left,
        test,
        right,
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// The test a comparison makes, by its mark: `!=`, `<=`, `>=`, `=`, `<` or `>`.
pub(super) fn test(input: &mut Input<'_>) -> ModalResult<WrittenTest> {
    alt((
        mark(Mark::Different).value(WrittenTest::NotEqual),
        mark(Mark::AtMost).value(WrittenTest::LessThanOrEqual),
        mark(Mark::AtLeast).value(WrittenTest::GreaterThanOrEqual),
        '='.value(WrittenTest::Equal),
        '<'.value(WrittenTest::LessThan),
        '>'.value(WrittenTest::GreaterThan),
        (keyword(Keyword::Not), multispace1, keyword(Keyword::In)).value(WrittenTest::NotIn),
        keyword(Keyword::In).value(WrittenTest::In),
    ))
    .parse_next(input)
}

/// One side of a comparison: a parameter, a text, a number, a function's call or a column.
pub(super) fn operand(input: &mut Input<'_>) -> ModalResult<WrittenOperand> {
    alt((
        parameter.map(WrittenOperand::Parameter),
        string.map(WrittenOperand::Text),
        number.map(WrittenOperand::Number),
        function.map(WrittenOperand::Function),
        column_reference.map(WrittenOperand::Column),
    ))
    .parse_next(input)
}

/// `concat(a, b)`, `pascal(pkg)`: a function applied to operands.
pub(super) fn function(input: &mut Input<'_>) -> ModalResult<WrittenApplied> {
    let ((function, arguments), span) = (
        terminated(
            alt((
                keyword(Keyword::Concat).value(WrittenFunction::Concat),
                keyword(Keyword::Pascal).value(WrittenFunction::Pascal),
                keyword(Keyword::Camel).value(WrittenFunction::Camel),
                keyword(Keyword::Snake).value(WrittenFunction::Snake),
                keyword(Keyword::Kebab).value(WrittenFunction::Kebab),
            )),
            (multispace0, '(', multispace0),
        ),
        terminated(
            separated(1.., operand, (multispace0, ',', multispace0)),
            (multispace0, ')'),
        ),
    )
        .with_span()
        .parse_next(input)?;
    Ok(WrittenApplied {
        function,
        arguments,
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// `alias.column`, or `column`; a column is a word or a string.
pub(super) fn column_reference(input: &mut Input<'_>) -> ModalResult<ColumnReference> {
    alt((
        (terminated(word, '.'), name_or_string).map(|(alias, column)| ColumnReference {
            alias: Alias::Written(AliasName(Box::from(alias.0))),
            column: ColumnName(column.0),
        }),
        name_or_string.map(|column| ColumnReference {
            alias: Alias::Unwritten,
            column: ColumnName(column.0),
        }),
    ))
    .parse_next(input)
}

/// `name(argument=value, ...)`.
pub(super) fn call(input: &mut Input<'_>) -> ModalResult<Call> {
    let ((name, arguments), span) = (
        terminated(word, (multispace0, '(', multispace0)),
        terminated(
            separated(0.., argument, (multispace0, ',', multispace0)),
            (multispace0, opt((',', multispace0)), ')'),
        ),
    )
        .with_span()
        .parse_next(input)?;
    Ok(Call {
        name: OperationName(Box::from(name.0)),
        arguments,
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// One named argument of a call, `name = value` or `name: value`.
pub(super) fn argument(input: &mut Input<'_>) -> ModalResult<Argument> {
    let (name, value) = (
        terminated(name_or_string, (multispace0, alt(('=', ':')), multispace0)),
        value,
    )
        .parse_next(input)?;
    Ok(Argument {
        name: ArgumentName(name.0),
        value,
    })
}

/// The value of an argument: a text, a number, a parameter, a list, a map or a word.
pub(super) fn value(input: &mut Input<'_>) -> ModalResult<ArgumentValue> {
    alt((
        string.map(ArgumentValue::Text),
        number.map(ArgumentValue::Number),
        parameter.map(ArgumentValue::Parameter),
        delimited(
            ('[', multispace0),
            separated(0.., value, (multispace0, ',', multispace0)),
            (multispace0, ']'),
        )
        .map(ArgumentValue::List),
        delimited(
            ('{', multispace0),
            separated(0.., argument, (multispace0, ',', multispace0)),
            (multispace0, '}'),
        )
        .map(ArgumentValue::Map),
        word.map(|word| ArgumentValue::Word(WordText(Box::from(word.0)))),
    ))
    .parse_next(input)
}

/// `$name`.
pub(super) fn parameter(input: &mut Input<'_>) -> ModalResult<WrittenParameter> {
    preceded('$', word)
        .map(|name| WrittenParameter(Box::from(name.0)))
        .parse_next(input)
}

/// A whole number.
pub(super) fn number(input: &mut Input<'_>) -> ModalResult<Integer> {
    digit1
        .try_map(str::parse::<i64>)
        .map(Integer)
        .parse_next(input)
}

/// A word, or a string in quotes: how a column or a key is written.
pub(super) fn name_or_string(input: &mut Input<'_>) -> ModalResult<WrittenName> {
    alt((
        string.map(|text| WrittenName(text.0)),
        word.map(|word| WrittenName(Box::from(word.0))),
    ))
    .parse_next(input)
}

/// Text in double quotes.
pub(super) fn string(input: &mut Input<'_>) -> ModalResult<WrittenLiteral> {
    delimited('"', take_till(0.., '"'), '"')
        .map(Box::<str>::from)
        .map(WrittenLiteral)
        .parse_next(input)
}

/// Letters, digits, `_` and `-`: `trait-files`, `max-lines`, `emit`.
pub(super) fn word<'i>(input: &mut Input<'i>) -> ModalResult<BareWord<'i>> {
    take_while(1.., ('a'..='z', 'A'..='Z', '0'..='9', '_', '-'))
        .map(BareWord)
        .parse_next(input)
}

/// A word that must be exactly `expected`.
fn keyword<'i>(
    expected: Keyword,
) -> impl Parser<Input<'i>, BareWord<'i>, winnow::error::ErrMode<winnow::error::ContextError>> {
    word.verify(move |found: &BareWord<'_>| found.0 == keyword_text(expected))
}

/// A mark of more than one character.
fn mark<'i>(
    expected: Mark,
) -> impl Parser<Input<'i>, BareWord<'i>, winnow::error::ErrMode<winnow::error::ContextError>> {
    mark_text(expected).map(BareWord)
}
