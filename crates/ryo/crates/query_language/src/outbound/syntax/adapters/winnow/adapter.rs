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
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::descriptions::types::{
    Example, Grammar, Production, ProductionForm, ProductionName,
};
use ryo_values::domain::program::types::{
    Alias, AliasName, Argument, ArgumentName, ArgumentValue, BareWord, ByteOffset, Call,
    ColumnName, ColumnReference, Columns, Condition, Expression, Filter, FragmentInput,
    FragmentProgram, FragmentReturn, ImportDeclaration, Join, JoinKey, Joining, LetStep,
    OperationName, ParameterDeclaration, Program, QuerySource, QuerySpan, QueryView,
    RelationSignature, Renaming, Select, SelectColumn, SignatureColumn, SourceReference,
    StepIdentity, StepName, TypeName, WordText, WrittenApplied, WrittenEmit, WrittenFunction,
    WrittenLiteral, WrittenName, WrittenObservation, WrittenOperand, WrittenParameter,
    WrittenRelation, WrittenTest,
};

/// The winnow syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxWinnowAdapter;

/// What the parsers read: the text, which also knows where each parser is in it.
type Input<'i> = LocatingSlice<&'i str>;

/// One line of a program, before the lines are put together.
pub(super) enum ProgramLine {
    /// An import declaration.
    Import(ImportDeclaration),
    /// A `param` line: a parameter the program is given.
    Parameter(ParameterDeclaration),
    /// A `let` line: a step that makes rows.
    Step(LetStep),
    /// A separately named semantic output.
    Observe(WrittenObservation),
    /// The `emit` line: the step whose rows are the findings.
    Emit(WrittenEmit),
}

/// One fragment module line before the module document is assembled.
pub(super) enum FragmentLine {
    /// A module dependency declaration.
    Import(ImportDeclaration),
    /// A declared relation input.
    Input(FragmentInput),
    /// A scalar parameter declaration.
    Parameter(ParameterDeclaration),
    /// The module output signature and its source span.
    Output(RelationSignature, QuerySpan),
    /// A module-local step definition.
    Step(LetStep),
    /// The returned module step.
    Return(FragmentReturn),
}

impl SyntaxPort for SyntaxWinnowAdapter {
    fn read(&self, text: QueryView<'_>) -> Result<Program, SyntaxError> {
        program
            .parse(LocatingSlice::new(text.0))
            .map_err(|error| unreadable(&error))
    }

    fn read_bundle(
        &self,
        bundle: &ryo_values::domain::program::types::QueryBundle,
    ) -> Result<ryo_values::domain::program::types::ParsedQueryBundle, SyntaxError> {
        use ryo_values::domain::program::types::{ParsedQueryBundle, QueryOrigin};
        let root_id = bundle.root.id.clone();
        let root = self
            .read(QueryView(&bundle.root.text))
            .map_err(|error| match error {
                SyntaxError::Unreadable { span, message } => SyntaxError::Located {
                    origin: QueryOrigin {
                        source_id: root_id.clone(),
                        span,
                        trace: Vec::new(),
                    },
                    message,
                },
                SyntaxError::Located { .. } => error,
            })?;
        let mut fragments = std::collections::BTreeMap::new();
        for (source_id, source) in &bundle.modules {
            let fragment = fragment_program
                .parse(LocatingSlice::new(&source.text))
                .map_err(|error| {
                    let said = error.inner().to_string();
                    SyntaxError::Located {
                        origin: QueryOrigin {
                            source_id: source_id.clone(),
                            span: QuerySpan {
                                start: ByteOffset(error.offset()),
                                end: ByteOffset(error.offset()),
                            },
                            trace: Vec::new(),
                        },
                        message: if said.is_empty() {
                            expectation().to_owned()
                        } else {
                            said
                        },
                    }
                })?;
            let _previous_fragment = fragments.insert(source_id.clone(), fragment);
        }
        Ok(ParsedQueryBundle { root, fragments })
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

/// Converts a parser failure into a source span and user-facing message.
fn unreadable(
    error: &winnow::error::ParseError<Input<'_>, winnow::error::ContextError>,
) -> SyntaxError {
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
}

/// The whole text: lines until the end, put together as parameters, steps and the one `emit`.
pub(super) fn program(input: &mut Input<'_>) -> ModalResult<Program> {
    let mut parameters = Vec::new();
    let mut imports = Vec::new();
    let mut steps = Vec::new();
    let mut emit = None;
    let mut observations = Vec::new();
    loop {
        multispace0.void().parse_next(input)?;
        if opt(eof).parse_next(input)?.is_some() {
            break;
        }
        match line.parse_next(input)? {
            ProgramLine::Import(import) => imports.push(import),
            ProgramLine::Parameter(parameter) => parameters.push(parameter),
            ProgramLine::Step(step) => steps.push(step),
            ProgramLine::Observe(output) => observations.push(output),
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
        Some(mut emit) => {
            assign_step_identities(&mut steps, &mut emit.step);
            for output in &mut observations {
                output.step.1 = steps
                    .iter()
                    .rev()
                    .find(|step| step.name.0 == output.step.0)
                    .and_then(|step| step.name.1);
            }
            Ok(Program {
                linked: false,
                imports,
                origins: Vec::new(),
                fragment_contracts: Vec::new(),
                step_origins: std::collections::BTreeMap::new(),
                observations,
                emit_origin: None,
                parameters,
                steps,
                emit,
            })
        }
        None => Err(winnow::error::ErrMode::Cut(
            winnow::error::ContextError::new(),
        )),
    }
}

/// A fragment module document: signature, steps and the one returned step.
pub(super) fn fragment_program(input: &mut Input<'_>) -> ModalResult<FragmentProgram> {
    let mut imports = Vec::new();
    let mut inputs = Vec::new();
    let mut parameters = Vec::new();
    let mut output = None;
    let mut output_span = None;
    let mut steps = Vec::new();
    let mut result = None;
    loop {
        multispace0.void().parse_next(input)?;
        if opt(eof).parse_next(input)?.is_some() {
            break;
        }
        match fragment_line.parse_next(input)? {
            FragmentLine::Import(value) => imports.push(value),
            FragmentLine::Input(value) => inputs.push(value),
            FragmentLine::Parameter(value) => parameters.push(value),
            FragmentLine::Output(value, span) => {
                if output.replace(value).is_some() || output_span.replace(span).is_some() {
                    return Err(winnow::error::ErrMode::Cut(
                        winnow::error::ContextError::new(),
                    ));
                }
            }
            FragmentLine::Step(value) => steps.push(value),
            FragmentLine::Return(value) => {
                if result.replace(value).is_some() {
                    return Err(winnow::error::ErrMode::Cut(
                        winnow::error::ContextError::new(),
                    ));
                }
            }
        }
    }
    match (output, output_span, result) {
        (Some(output), Some(output_span), Some(mut result)) => {
            assign_step_identities(&mut steps, &mut result.step);
            Ok(FragmentProgram {
                imports,
                inputs,
                parameters,
                output,
                output_span,
                steps,
                result,
            })
        }
        _ => Err(winnow::error::ErrMode::Cut(
            winnow::error::ContextError::new(),
        )),
    }
}

/// Gives each definition a source-local identity and binds preceding-step references to that
/// identity. Unknown and forward references remain unresolved for validation.
/// Assigns each local step an identity and resolves only preceding local references.
fn assign_step_identities(steps: &mut [LetStep], result: &mut StepName) {
    for (place, step) in steps.iter_mut().enumerate() {
        step.name.1 = Some(StepIdentity(u64::try_from(place).unwrap_or(u64::MAX)));
    }
    for place in 0..steps.len() {
        let (preceding, current) = steps.split_at_mut(place);
        if let Some(step) = current.first_mut() {
            match &mut step.expression {
                Expression::Select(select) => {
                    resolve_source_step(&mut select.from, preceding);
                    if let Joining::Joined(join) = &mut select.join {
                        resolve_source_step(&mut join.source, preceding);
                    }
                }
                Expression::Call(_) => {}
            }
        }
    }
    result.1 = steps
        .iter()
        .rev()
        .find(|candidate| candidate.name.0 == result.0)
        .and_then(|candidate| candidate.name.1);
}

/// Resolves a step source only when an earlier definition with that name exists.
fn resolve_source_step(source: &mut QuerySource, preceding: &[LetStep]) {
    if let SourceReference::Step(reference) = &mut source.reference {
        reference.1 = preceding
            .iter()
            .rev()
            .find(|candidate| candidate.name.0 == reference.0)
            .and_then(|candidate| candidate.name.1);
    }
}

/// Parses one import, signature, declaration, step, or return line in a module.
pub(super) fn fragment_line(input: &mut Input<'_>) -> ModalResult<FragmentLine> {
    alt((
        import_line.map(FragmentLine::Import),
        fragment_input_line.map(FragmentLine::Input),
        parameter_line.map(FragmentLine::Parameter),
        fragment_output_line.map(|(signature, span)| FragmentLine::Output(signature, span)),
        step_line.map(FragmentLine::Step),
        fragment_return_line.map(FragmentLine::Return),
    ))
    .parse_next(input)
}

/// `input candidates: relation<id: node, file: file, span: span>;`.
pub(super) fn fragment_input_line(input: &mut Input<'_>) -> ModalResult<FragmentInput> {
    let ((name, relation), span) = delimited(
        (keyword(Keyword::Input), multispace1),
        (
            terminated(word, (multispace0, ':', multispace0)),
            relation_signature,
        ),
        (multispace0, ';'),
    )
    .with_span()
    .parse_next(input)?;
    Ok(FragmentInput {
        name: WrittenName(Box::from(name.0)),
        relation,
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// `output relation<id: node, file: file, span: span>;`.
pub(super) fn fragment_output_line(
    input: &mut Input<'_>,
) -> ModalResult<(RelationSignature, QuerySpan)> {
    delimited(
        (keyword(Keyword::Output), multispace1),
        relation_signature,
        (multispace0, ';'),
    )
    .with_span()
    .map(|(signature, span)| {
        (
            signature,
            QuerySpan {
                start: ByteOffset(span.start),
                end: ByteOffset(span.end),
            },
        )
    })
    .parse_next(input)
}

/// Parses an ordered relation signature with byte spans for each declared column.
pub(super) fn relation_signature(input: &mut Input<'_>) -> ModalResult<RelationSignature> {
    let (columns, span) = delimited(
        (keyword(Keyword::Relation), multispace0, '<', multispace0),
        separated(
            1..,
            (terminated(word, (multispace0, ':', multispace0)), word)
                .with_span()
                .map(|((name, kind), span)| SignatureColumn {
                    name: ColumnName(Box::from(name.0)),
                    kind: TypeName(Box::from(kind.0)),
                    span: QuerySpan {
                        start: ByteOffset(span.start),
                        end: ByteOffset(span.end),
                    },
                }),
            (multispace0, ',', multispace0),
        ),
        (multispace0, '>'),
    )
    .with_span()
    .parse_next(input)?;
    Ok(RelationSignature {
        columns,
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// `return step;`.
pub(super) fn fragment_return_line(input: &mut Input<'_>) -> ModalResult<FragmentReturn> {
    let (step, span) = delimited(
        (keyword(Keyword::Return), multispace1),
        word.map(|name| StepName(Box::from(name.0), None)),
        (multispace0, ';'),
    )
    .with_span()
    .parse_next(input)?;
    Ok(FragmentReturn {
        step,
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// One line of a program: a parameter declaration, a `let` step or an `emit`.
pub(super) fn line(input: &mut Input<'_>) -> ModalResult<ProgramLine> {
    alt((
        import_line.map(ProgramLine::Import),
        parameter_line.map(ProgramLine::Parameter),
        step_line.map(ProgramLine::Step),
        observe_line.map(ProgramLine::Observe),
        emit_line.map(ProgramLine::Emit),
    ))
    .parse_next(input)
}

/// `observe matched as "usage";`: a semantic output that never becomes a diagnostic by itself.
pub(super) fn observe_line(input: &mut Input<'_>) -> ModalResult<WrittenObservation> {
    let ((step, name), span) = delimited(
        (keyword(Keyword::Observe), multispace1),
        (
            terminated(word, (multispace1, keyword(Keyword::As), multispace1)),
            name_or_string,
        ),
        (multispace0, ';'),
    )
    .with_span()
    .parse_next(input)?;
    Ok(WrittenObservation {
        step: StepName(step.0.into(), None),
        name: ryo_values::domain::observations::types::ObservationName(name.0.into()),
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
        origin: None,
    })
}

/// `import suppress from "python/ruff/suppress";`.
pub(super) fn import_line(input: &mut Input<'_>) -> ModalResult<ImportDeclaration> {
    let ((alias, module), span) = delimited(
        (keyword(Keyword::Import), multispace1),
        (
            terminated(word, (multispace1, keyword(Keyword::From), multispace1)),
            string,
        ),
        (multispace0, ';'),
    )
    .with_span()
    .parse_next(input)?;
    Ok(ImportDeclaration {
        alias: WrittenName(Box::from(alias.0)),
        module: WrittenLiteral(module.0),
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
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
        name: StepName(Box::from(name.0), None),
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
        step: StepName(Box::from(step.0), None),
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
            word.map(|name| SourceReference::Step(StepName(Box::from(name.0), None))),
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
        numeric_filter,
        integer_successor_filter,
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
                keyword(Keyword::NaturalLt).value(WrittenTest::NaturalLessThan),
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

/// `numeric_gte(left, right)`.
pub(super) fn numeric_filter(input: &mut Input<'_>) -> ModalResult<Filter> {
    let ((left, right), span) = (
        preceded(
            (keyword(Keyword::NumericGte), multispace0, '(', multispace0),
            terminated(operand, (multispace0, ',', multispace0)),
        ),
        terminated(operand, (multispace0, ')')),
    )
        .with_span()
        .parse_next(input)?;
    Ok(Filter {
        left,
        test: WrittenTest::NumericGreaterThanOrEqual,
        right,
        span: QuerySpan {
            start: ByteOffset(span.start),
            end: ByteOffset(span.end),
        },
    })
}

/// `integer_successor(left, right)`.
pub(super) fn integer_successor_filter(input: &mut Input<'_>) -> ModalResult<Filter> {
    let ((left, right), span) = (
        preceded(
            (
                keyword(Keyword::IntegerSuccessor),
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
        test: WrittenTest::IntegerSuccessor,
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
                keyword(Keyword::Lower).value(WrittenFunction::Lower),
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
