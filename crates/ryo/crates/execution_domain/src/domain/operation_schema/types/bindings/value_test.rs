//! Binding schemas check configuration without creating values in execution storage.
use std::borrow::Cow;

use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::bindings::value::BindingShape;
use ryo_relations::domain::relations::types::{CellKind, Integer};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, FlagState, LiteralText, Operand, ParameterId, PlanComparison, QueryLiteral,
    TableKey, Test, WholeNumber,
};

/// What a row asks of the shape of the parameters bound.
enum Ask {
    /// The kinds of columns these values extend columns of these kinds with.
    Extend(Vec<ParameterBinding>, Vec<Operand>, Vec<CellKind>),
    /// The kinds of the columns of the rows a parameter is bound to, read by these keys.
    Rows(Vec<ParameterBinding>, ParameterId, &'static [&'static str]),
    /// Whether a regex match of a symbol column against this operand checks.
    Predicate(Vec<ParameterBinding>, Operand),
}

/// What the shape answers.
#[derive(Debug, PartialEq)]
enum Answer {
    /// The kinds of the columns.
    Kinds(Cow<'static, [CellKind]>),
    /// The predicates check.
    Checked,
}

type Input = Ask;
type Value = Answer;
type Failure = OperatingError;

fn subject(ask: Input) -> Result<Value, Failure> {
    Ok(match ask {
        Ask::Extend(bindings, values, kinds) => Answer::Kinds(Cow::Owned(
            BindingShape::new(&bindings).extended(&values, kinds)?,
        )),
        Ask::Rows(bindings, parameter, keys) => {
            let keys: Vec<TableKey> = keys.iter().map(|key| TableKey((*key).into())).collect();
            Answer::Kinds(Cow::Owned(
                BindingShape::new(&bindings).rows(parameter, &keys)?,
            ))
        }
        Ask::Predicate(bindings, right) => {
            let comparison = PlanComparison {
                test: Test::Matches,
                left: Operand::Column(ColumnIndex(0)),
                right,
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            };
            BindingShape::new(&bindings).predicates(&[comparison], &[CellKind::Symbol])?;
            Answer::Checked
        }
    })
}

/// A text, a number and a flag.
fn scalars() -> Vec<ParameterBinding> {
    vec![
        ParameterBinding::Text(LiteralText("never-intern".into())),
        ParameterBinding::Number(WholeNumber(2)),
        ParameterBinding::Flag(FlagState::Raised),
    ]
}

/// An empty list of texts, of numbers and of text sets.
fn empty_lists() -> Vec<ParameterBinding> {
    vec![
        ParameterBinding::Texts(Vec::new()),
        ParameterBinding::Numbers(Vec::new()),
        ParameterBinding::TextSets(Vec::new()),
    ]
}

/// One table of one row whose cell `name` holds an unread value.
fn named_table() -> Vec<ParameterBinding> {
    vec![ParameterBinding::Tables(vec![BindingTable(vec![
        BindingCell {
            key: LiteralText("name".into()),
            value: LiteralText("unread-value".into()),
        },
    ])])]
}

/// An invalid regex, bound as text.
fn bound_regex() -> Vec<ParameterBinding> {
    vec![ParameterBinding::Text(LiteralText("[".into()))]
}

/// Configuration failures are independent of row cardinality; dynamic patterns are not evaluated.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("scalar_extensions_preserve_types"),
        word: Word::Succeeds,
        input: || {
            Ask::Extend(
                scalars(),
                vec![
                    Operand::Parameter(ParameterId(0)),
                    Operand::Parameter(ParameterId(1)),
                    Operand::Parameter(ParameterId(2)),
                    Operand::Literal(QueryLiteral::Number(Integer(4))),
                ],
                vec![CellKind::Node],
            )
        },
        expect: Expect::Value(Answer::Kinds(Cow::Borrowed(&[
            CellKind::Node,
            CellKind::Symbol,
            CellKind::Number,
            CellKind::Flag,
            CellKind::Number,
        ]))),
    },
    Case {
        name: CaseName("all_empty_list_encodings_are_valid_rows"),
        word: Word::Succeeds,
        input: || Ask::Rows(empty_lists(), ParameterId(0), &["name"]),
        expect: Expect::Value(Answer::Kinds(Cow::Borrowed(&[CellKind::Symbol]))),
    },
    Case {
        name: CaseName("an_empty_number_list_is_valid_rows"),
        word: Word::Succeeds,
        input: || Ask::Rows(empty_lists(), ParameterId(1), &["name"]),
        expect: Expect::Value(Answer::Kinds(Cow::Borrowed(&[CellKind::Symbol]))),
    },
    Case {
        name: CaseName("an_empty_text_set_list_is_valid_rows"),
        word: Word::Succeeds,
        input: || Ask::Rows(empty_lists(), ParameterId(2), &["name"]),
        expect: Expect::Value(Answer::Kinds(Cow::Borrowed(&[CellKind::Symbol]))),
    },
    Case {
        name: CaseName("repeated_keys_preserve_column_order"),
        word: Word::Succeeds,
        input: || Ask::Rows(named_table(), ParameterId(0), &["name", "name"]),
        expect: Expect::Value(Answer::Kinds(Cow::Borrowed(&[CellKind::Symbol; 2]))),
    },
    Case {
        name: CaseName("missing_table_key_is_refused"),
        word: Word::Refuses,
        input: || Ask::Rows(named_table(), ParameterId(0), &["missing", "missing"]),
        expect: Expect::Error(OperatingError::WrongBinding),
    },
    Case {
        name: CaseName("unbound_table_parameter_is_refused"),
        word: Word::Refuses,
        input: || Ask::Rows(Vec::new(), ParameterId(0), &[]),
        expect: Expect::Error(OperatingError::Unbound),
    },
    Case {
        name: CaseName("collection_cannot_be_scalar_extension"),
        word: Word::Refuses,
        input: || {
            Ask::Extend(
                vec![ParameterBinding::Texts(Vec::new())],
                vec![Operand::Parameter(ParameterId(0))],
                Vec::new(),
            )
        },
        expect: Expect::Error(OperatingError::WrongBinding),
    },
    Case {
        name: CaseName("nonempty_text_list_cannot_be_tables"),
        word: Word::Refuses,
        input: || {
            Ask::Rows(
                vec![ParameterBinding::Texts(vec![LiteralText(
                    "not-table".into(),
                )])],
                ParameterId(0),
                &[],
            )
        },
        expect: Expect::Error(OperatingError::WrongBinding),
    },
    Case {
        name: CaseName("bound_regex_content_remains_execution_owned"),
        word: Word::Succeeds,
        input: || Ask::Predicate(bound_regex(), Operand::Parameter(ParameterId(0))),
        expect: Expect::Value(Answer::Checked),
    },
    Case {
        name: CaseName("dynamic_regex_remains_execution_owned"),
        word: Word::Succeeds,
        input: || Ask::Predicate(bound_regex(), Operand::Column(ColumnIndex(0))),
        expect: Expect::Value(Answer::Checked),
    },
    Case {
        name: CaseName("predicate_binding_errors_remain_explicit"),
        word: Word::Refuses,
        input: || Ask::Predicate(bound_regex(), Operand::Parameter(ParameterId(9))),
        expect: Expect::Error(OperatingError::Unbound),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::query_bindings::types::{BindingCell, BindingTable, ParameterBinding};
