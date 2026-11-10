//! Binding schemas check configuration without creating values in execution storage.

use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::bindings::value::BindingShape;
use ryo_relations::domain::relations::types::{CellKind, Integer};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, FlagState, LiteralText, Operand, ParameterId, PlanComparison, QueryLiteral,
    TableKey, Test, WholeNumber,
};

/// Each variant asserts a binding-specific contract, with no operator payload inputs.
#[derive(Clone, Copy)]
enum Scenario {
    Scalars,
    EmptyLists,
    Keys,
    MissingKey,
    Unbound,
    WrongScalar,
    WrongRows,
    InvalidBoundRegex,
    DynamicRegex,
    PredicateBinding,
}
/// The binding contract exercised by a case.
type Input = Scenario;
/// The number of output columns or explicit successful validations.
type Value = usize;
type Failure = OperatingError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Scalars => {
            let bindings = [
                ParameterBinding::Text(LiteralText("never-intern".into())),
                ParameterBinding::Number(WholeNumber(2)),
                ParameterBinding::Flag(FlagState::Raised),
            ];
            let values = [
                Operand::Parameter(ParameterId(0)),
                Operand::Parameter(ParameterId(1)),
                Operand::Parameter(ParameterId(2)),
                Operand::Literal(QueryLiteral::Number(Integer(4))),
            ];
            let result = BindingShape::new(&bindings).extended(&values, vec![CellKind::Node])?;
            assert_eq!(
                result,
                [
                    CellKind::Node,
                    CellKind::Symbol,
                    CellKind::Number,
                    CellKind::Flag,
                    CellKind::Number
                ]
            );
            Ok(result.len())
        }
        Scenario::EmptyLists => {
            let bindings = [
                ParameterBinding::Texts(Vec::new()),
                ParameterBinding::Numbers(Vec::new()),
                ParameterBinding::TextSets(Vec::new()),
            ];
            for index in 0..3 {
                assert_eq!(
                    BindingShape::new(&bindings)
                        .rows(ParameterId(index), &[TableKey("name".into())])?,
                    [CellKind::Symbol]
                );
            }
            Ok(3)
        }
        Scenario::Keys | Scenario::MissingKey => {
            let bindings = [ParameterBinding::Tables(vec![BindingTable(vec![
                BindingCell {
                    key: LiteralText("name".into()),
                    value: LiteralText("unread-value".into()),
                },
            ])])];
            let key = if matches!(scenario, Scenario::Keys) {
                "name"
            } else {
                "missing"
            };
            let kinds = BindingShape::new(&bindings).rows(
                ParameterId(0),
                &[TableKey(key.into()), TableKey(key.into())],
            )?;
            assert_eq!(kinds, [CellKind::Symbol; 2]);
            Ok(kinds.len())
        }
        Scenario::Unbound => BindingShape::new(&[])
            .rows(ParameterId(0), &[])
            .map(|result| result.len()),
        Scenario::WrongScalar => BindingShape::new(&[ParameterBinding::Texts(Vec::new())])
            .extended(&[Operand::Parameter(ParameterId(0))], Vec::new())
            .map(|result| result.len()),
        Scenario::WrongRows => BindingShape::new(&[ParameterBinding::Texts(vec![LiteralText(
            "not-table".into(),
        )])])
        .rows(ParameterId(0), &[])
        .map(|result| result.len()),
        Scenario::InvalidBoundRegex | Scenario::DynamicRegex | Scenario::PredicateBinding => {
            let bindings = [ParameterBinding::Text(LiteralText("[".into()))];
            let right = match scenario {
                Scenario::DynamicRegex => Operand::Column(ColumnIndex(0)),
                Scenario::PredicateBinding => Operand::Parameter(ParameterId(9)),
                Scenario::InvalidBoundRegex
                | Scenario::Scalars
                | Scenario::EmptyLists
                | Scenario::Keys
                | Scenario::MissingKey
                | Scenario::Unbound
                | Scenario::WrongScalar
                | Scenario::WrongRows => Operand::Parameter(ParameterId(0)),
            };
            let comparison = PlanComparison {
                test: Test::Matches,
                left: Operand::Column(ColumnIndex(0)),
                right,
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            };
            BindingShape::new(&bindings).predicates(&[comparison], &[CellKind::Symbol])?;
            Ok(1)
        }
    }
}

/// Configuration failures are independent of row cardinality; dynamic patterns are not evaluated.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("scalar_extensions_preserve_types"),
        word: Word::Succeeds,
        input: || Scenario::Scalars,
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("all_empty_list_encodings_are_valid_rows"),
        word: Word::Succeeds,
        input: || Scenario::EmptyLists,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("repeated_keys_preserve_column_order"),
        word: Word::Succeeds,
        input: || Scenario::Keys,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("missing_table_key_is_refused"),
        word: Word::Refuses,
        input: || Scenario::MissingKey,
        expect: Expect::Error(OperatingError::WrongBinding),
    },
    Case {
        name: CaseName("unbound_table_parameter_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Unbound,
        expect: Expect::Error(OperatingError::Unbound),
    },
    Case {
        name: CaseName("collection_cannot_be_scalar_extension"),
        word: Word::Refuses,
        input: || Scenario::WrongScalar,
        expect: Expect::Error(OperatingError::WrongBinding),
    },
    Case {
        name: CaseName("nonempty_text_list_cannot_be_tables"),
        word: Word::Refuses,
        input: || Scenario::WrongRows,
        expect: Expect::Error(OperatingError::WrongBinding),
    },
    Case {
        name: CaseName("bound_regex_content_remains_execution_owned"),
        word: Word::Succeeds,
        input: || Scenario::InvalidBoundRegex,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("dynamic_regex_remains_execution_owned"),
        word: Word::Succeeds,
        input: || Scenario::DynamicRegex,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("predicate_binding_errors_remain_explicit"),
        word: Word::Refuses,
        input: || Scenario::PredicateBinding,
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

/// Compare actual evaluator errors on empty/populated Stored inputs with metadata-only success.
use ryo_values::domain::query_bindings::types::{BindingCell, BindingTable, ParameterBinding};
