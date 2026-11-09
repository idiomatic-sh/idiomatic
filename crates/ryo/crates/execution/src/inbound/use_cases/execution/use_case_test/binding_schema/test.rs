//! Invalid bound and row patterns stay lazy until actual rows reach the public owner.
use super::super::super::fixtures;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, LiteralText, Operand, ParameterId, PlanComparison, Test},
    query_bindings::types::ParameterBinding,
    rows::types::{RowCount, Table},
};
#[derive(Clone, Copy)]
enum Scenario {
    BoundExecution,
    DynamicExecution,
}
type Input = Scenario;
type Value = usize;
type Failure = OperatingError;
fn execution_boundary(scenario: Scenario) -> Result<Value, Failure> {
    use ryo_execution_domain::domain::operation_schema::{service::schema_of, types::InputShape};
    use ryo_language::domain::symbols::{
        service::intern_derived,
        types::{ByteSlice, Symbols},
    };
    use ryo_relations::domain::relations::types::Value as Cell;
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
    let comparison = PlanComparison {
        test: Test::Matches,
        left: Operand::Column(ColumnIndex(0)),
        right: if dynamic {
            Operand::Column(ColumnIndex(1))
        } else {
            Operand::Parameter(ParameterId(0))
        },
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
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
    let empty = Table {
        kinds: kinds.to_vec(),
        ..Table::default()
    };
    let answer = fixtures::requests::table_request(
        operation.clone(),
        &[&empty],
        bindings.to_vec(),
        &mut symbols,
    )?;
    assert_eq!(answer.count.0, 0);
    let populated = Table {
        kinds: kinds.to_vec(),
        count: RowCount(1),
        cells: if dynamic {
            vec![Cell::Symbol(word), Cell::Symbol(bad)]
        } else {
            vec![Cell::Symbol(word)]
        },
    };
    let reached = fixtures::requests::table_request(
        operation,
        &[&populated],
        bindings.to_vec(),
        &mut symbols,
    );
    assert!(matches!(reached, Err(OperatingError::InvalidRegex { .. })));
    Ok(2)
}

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
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    execution_boundary(scenario)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

use ryo_relations::domain::relations::types::CellKind;

use ryo_relations::domain::relations::types::Relation;
