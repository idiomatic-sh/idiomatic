//! Binding schemas check configuration without creating values in execution storage.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::bindings::value::BindingShape;
use crate::domain::table_model::types::{};
use ryo_relations::domain::relations::types::{CellKind, Integer};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, FlagState, LiteralText, Operand, ParameterId, QueryLiteral, TableKey,
    Test, WholeNumber,
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
    BoundExecution,
    DynamicExecution,
}
/// The binding contract exercised by a case.
type Input = Scenario;
/// The number of output columns or explicit successful validations.
type Value = usize;
type Failure = OperatingError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::BoundExecution | Scenario::DynamicExecution => {
            return execution_boundary(scenario);
        }
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
                _ => Operand::Parameter(ParameterId(0)),
            };
            let comparison = Comparison {
                test: Test::Matches,
                left: Operand::Column(ColumnIndex(0)),
                right,
                origin: None,
            };
            BindingShape::new(&bindings).predicates(&[comparison], &[CellKind::Symbol])?;
            Ok(1)
        }
    }
}

/// Configuration failures are independent of row cardinality; dynamic patterns are not evaluated.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("bound_regex_fails_only_when_its_row_is_executed"),
        word: Word::Succeeds,
        input: || Scenario::BoundExecution,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("row_regex_fails_only_when_its_row_is_executed"),
        word: Word::Succeeds,
        input: || Scenario::DynamicExecution,
        expect: Expect::Value(2),
    },
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
fn execution_boundary(scenario: Scenario) -> Result<Value, Failure> {
    use crate::application::file_dag::cross_execution::compute;
    use crate::domain::operating::types::OperatorContext;
    use crate::domain::operation_schema::{service::schema_of, types::InputShape};
    use crate::domain::table_model::types::Rows;
    use crate::domain::table_scratch::types::OperatorScratch;
    use {ryo_values::domain::files::types::SourceView};
    use ryo_language::domain::symbols::{
        service::intern_derived,
        types::{ByteSlice, Symbols},
    };
    use ryo_relations::domain::relations::{
        service::to_bits,
        types::{ColumnsView, FactColumn, Relation, Value as Cell},
    };
    use ryo_relations::domain::terms::types::NameText;
    use ryo_values::domain::plans::types::Operation;
    let mut symbols = Symbols::default();
    let word = intern_derived(&mut symbols, ByteSlice(b""), NameText("word"));
    let bad = intern_derived(&mut symbols, ByteSlice(b""), NameText("["));
    let dynamic = matches!(scenario, Scenario::DynamicExecution);
    let kinds: &'static [CellKind] = if dynamic {
        &[CellKind::Symbol, CellKind::Symbol]
    } else {
        &[CellKind::Symbol]
    };
    let mut columns = vec![FactColumn(vec![to_bits(Cell::Symbol(word)).0])];
    if dynamic {
        columns.push(FactColumn(vec![to_bits(Cell::Symbol(bad)).0]));
    }
    let comparison = Comparison {
        test: Test::Matches,
        left: Operand::Column(ColumnIndex(0)),
        right: if dynamic {
            Operand::Column(ColumnIndex(1))
        } else {
            Operand::Parameter(ParameterId(0))
        },
        origin: None,
    };
    let operation = Operation::Select {
        from: ryo_values::domain::plan_parts::types::Input::Relation(Relation::Function),
        comparisons: vec![comparison],
        kept: Vec::new(),
    };
    let bindings = [ParameterBinding::Text(LiteralText("[".into()))];
    let metadata = [InputShape {
        kinds: &kinds,
        tree: None,
    }];
    assert_eq!(schema_of(&operation, &metadata, &bindings)?, kinds);
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let empty = Table {
        kinds: kinds.to_vec(),
        ..Table::default()
    };
    let mut answer = Table::default();
    compute(
        &operation,
        &[Rows::Computed(&empty)],
        &mut context,
        &mut answer,
    )?;
    assert_eq!(answer.count.0, 0);
    let reached = compute(
        &operation,
        &[Rows::Stored(ColumnsView {
            kinds,
            columns: &columns,
        })],
        &mut context,
        &mut answer,
    );
    assert!(matches!(reached, Err(OperatingError::InvalidRegex { .. })));
    Ok(2)
}

use ryo_values::domain::rows::types::Table;

use ryo_values::domain::query_bindings::types::{ParameterBinding,BindingTable,BindingCell};
