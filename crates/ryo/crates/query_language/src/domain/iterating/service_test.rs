//! A `worklist` call, from typed values: it lowers to one step with the trace's columns, and a state or order column of
//! another kind is refused with its fault.
use super::service::call_worklist;
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::ArgumentFault;
use crate::selects;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_syntax::types as ast;
use ryo_values::domain::query_tokens::types as tokens;
use std::borrow::Cow;

/// The arguments a case changes: the root's state column and the edges' order column.
#[derive(Clone, Copy, Debug)]
struct Columns {
    /// The roots' state column.
    state: &'static str,
    /// The edges' order column.
    order: &'static str,
}

type Input = Columns;
/// The step's operation and its columns, each `name:kind`.
type Value = Cow<'static, str>;
type Failure = LoweringError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_worklist_call_lowers_to_one_step_with_the_trace_columns"),
        word: Word::Succeeds,
        input: || Columns {
            state: "name",
            order: "ordinal",
        },
        expect: Expect::Value(Cow::Borrowed(
            "1 steps, worklist: file:File id:Node parent:Node child:Node ordinal:Number field:Symbol reached:Symbol",
        )),
    },
    Case {
        name: CaseName("a_state_column_of_another_kind_is_refused"),
        word: Word::Refuses,
        input: || Columns {
            state: "id",
            order: "ordinal",
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::MatchingStates,
                    ..
                }
            ))
        }),
    },
    Case {
        name: CaseName("a_non_number_order_is_refused"),
        word: Word::Refuses,
        input: || Columns {
            state: "name",
            order: "field",
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::NumericGraphOrder,
                    ..
                }
            ))
        }),
    },
];

/// A text argument.
fn text(value: &str) -> ast::ArgumentValue {
    ast::ArgumentValue::Text(tokens::WrittenLiteral(value.into()))
}

/// A word argument.
fn word(value: &str) -> ast::ArgumentValue {
    ast::ArgumentValue::Word(tokens::WordText(value.into()))
}

/// The worklist from each function in the state of its name, over the syntax tree's edges, the lattice of the function
/// names, keyed by file.
fn call(columns: Columns) -> ast::Call {
    let arguments = vec![
        ("from", text("function")),
        ("root", word("id")),
        ("state", word(columns.state)),
        ("edges", text("syntax-child")),
        ("source", word("parent")),
        ("destination", word("child")),
        ("order", word(columns.order)),
        ("before", word("field")),
        ("after", word("field")),
        ("lattice", text("function")),
        ("lower", word("name")),
        ("upper", word("name")),
        ("keys", ast::ArgumentValue::List(vec![word("file")])),
        ("output", word("reached")),
    ];
    ast::Call {
        name: tokens::OperationName("worklist".into()),
        span: lowering_builder().span,
        arguments: arguments
            .into_iter()
            .map(|(name, value)| ast::Argument {
                name: tokens::ArgumentName(name.into()),
                value,
            })
            .collect(),
    }
}

fn subject(columns: Input) -> Result<Value, Failure> {
    let mut builder = lowering_builder();
    let name = tokens::StepName("trace".into(), tokens::StepIdentified::Unassigned);
    call_worklist(&mut builder, &name, &call(columns))?;
    let operation = match builder.steps.last().map(|step| &step.operation) {
        Some(Operation::Worklist { .. }) => "worklist",
        _ => "another",
    };
    let fields = builder
        .lowered
        .last()
        .map(|lowered| {
            lowered
                .fields
                .iter()
                .map(|field| format!("{}:{:?}", field.name.0, field.kind))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    Ok(Cow::Owned(format!(
        "{} steps, {operation}: {fields}",
        builder.steps.len()
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
