//! Preparation rejects foreign operations and retains input/value errors for execution.
use std::borrow::Cow;

use super::service::prepare_operator;

const PER_FILE: ryo_values::domain::plan_parts::types::StepScope =
    ryo_values::domain::plan_parts::types::StepScope::PerFile;
use crate::domain::{operating::errors::OperatingError, operation_schema::types::InputShape};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Input as Source, Operand, RepeatedRows},
        plans::types::Operation,
    },
};
/// What a row asks of operator preparation.
enum Ask {
    /// Which operators the documentation lists, by how many, do not describe themselves with a name, a summary and an example.
    Described,
    /// Which kinds of `OperationKind::ALL`, by how many, do not refuse an operation of another kind as the wrong operation.
    ForeignToEveryKind,
    /// The contract a kind prepares for an operation over inputs of these kinds.
    Prepare(OperationKind, Operation, &'static [&'static [CellKind]]),
}
/// What preparation answers.
#[derive(Debug, PartialEq)]
enum Answer {
    /// How many there are, and those that fail.
    Census(usize, Cow<'static, [Cow<'static, str>]>),
    /// The contract's schema and output kinds.
    Contract {
        schema: Cow<'static, [CellKind]>,
        output: Cow<'static, [CellKind]>,
    },
}
type Input = Ask;
type Value = Answer;
type Failure = OperatingError;
/// A projection of `function`'s first column.
fn project() -> Operation {
    Operation::Project {
        from: Source::Relation(Relation::Function),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    }
}
/// A union of `function` with itself.
fn union() -> Operation {
    Operation::Union {
        left: Source::Relation(Relation::Function),
        right: Source::Relation(Relation::Function),
    }
}
/// The documented operators that do not describe themselves.
fn described() -> Value {
    let undescribed = super::constants::OPERATORS
        .iter()
        .filter(|operator| {
            let description = super::constants::describe(**operator);
            description.name.0.is_empty()
                || description.summary.0.is_empty()
                || !description.example.0.contains("example")
        })
        .map(|operator| Cow::Owned(format!("{operator:?}")))
        .collect();
    Answer::Census(super::constants::OPERATORS.len(), undescribed)
}
/// The kinds that prepare, or refuse otherwise, an operation of another kind.
fn foreign_to_every_kind() -> Value {
    let (union, project) = (union(), project());
    let accepting = OperationKind::ALL
        .iter()
        .filter(|kind| {
            let foreign = if **kind == OperationKind::Union {
                &project
            } else {
                &union
            };
            prepare_operator(**kind, foreign, &[], &[], PER_FILE).err()
                != Some(OperatingError::WrongOperation)
        })
        .map(|kind| Cow::Owned(format!("{kind:?}")))
        .collect();
    Answer::Census(OperationKind::ALL.len(), accepting)
}
fn subject(ask: Input) -> Result<Value, Failure> {
    Ok(match ask {
        Ask::Described => described(),
        Ask::ForeignToEveryKind => foreign_to_every_kind(),
        Ask::Prepare(kind, operation, inputs) => {
            let shapes: Vec<InputShape<'_>> = inputs
                .iter()
                .map(|kinds| InputShape { kinds, tree: None })
                .collect();
            let contract = prepare_operator(kind, &operation, &shapes, &[], PER_FILE)?;
            Answer::Contract {
                schema: Cow::Owned(contract.schema?),
                output: Cow::Owned(contract.output_kinds?),
            }
        }
    })
}
/// One input of one file column.
const FILES: &[&[CellKind]] = &[&[CellKind::File]];
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_operator_describes_itself"),
        word: Word::Succeeds,
        input: || Ask::Described,
        expect: Expect::Value(Answer::Census(25, Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("every_kind_refuses_an_operation_of_another_kind"),
        word: Word::Succeeds,
        input: || Ask::ForeignToEveryKind,
        expect: Expect::Value(Answer::Census(26, Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("a_valid_projection_retains_its_output_schema"),
        word: Word::Succeeds,
        input: || Ask::Prepare(OperationKind::Project, project(), FILES),
        expect: Expect::Value(Answer::Contract {
            schema: Cow::Borrowed(&[CellKind::File]),
            output: Cow::Borrowed(&[CellKind::File]),
        }),
    },
    Case {
        name: CaseName("an_operator_refuses_an_operation_that_is_not_its_own"),
        word: Word::Refuses,
        input: || Ask::Prepare(OperationKind::Project, union(), FILES),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("an_operator_requires_its_declared_input"),
        word: Word::Refuses,
        input: || Ask::Prepare(OperationKind::Project, project(), &[]),
        expect: Expect::Error(OperatingError::MissingInput { position: 1 }),
    },
    Case {
        name: CaseName("a_column_is_not_a_value_to_append"),
        word: Word::Refuses,
        input: || {
            Ask::Prepare(
                OperationKind::Extend,
                Operation::Extend {
                    from: Source::Relation(Relation::Function),
                    values: vec![Operand::Column(ColumnIndex(0))],
                },
                FILES,
            )
        },
        expect: Expect::Error(OperatingError::WrongBinding),
    },
];
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
