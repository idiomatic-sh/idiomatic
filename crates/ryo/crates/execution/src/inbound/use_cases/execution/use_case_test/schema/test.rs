//! Proves every operation schema against its actual empty and populated evaluator.

use super::super::super::fixtures;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::values::KnownSyntax;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_domain::domain::operation_schema::fixtures::{bindings, operations, tables};
use ryo_execution_domain::domain::operation_schema::service::schema_of;
use ryo_execution_domain::domain::operation_schema::types::InputShape;
use ryo_language::domain::symbols::{
    service::intern_derived,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plans::types::Operation;

/// What a row of the cases asks of the schemas.
#[derive(Clone, Copy)]
enum Scenario {
    /// Every operator's schema against its evaluator over empty tables.
    Empty,
    /// Every operator's schema against its evaluator over populated tables.
    Populated,
    /// A hosted relation published without its request source.
    HostedWithoutSource,
}

/// A genuine populated/empty evaluator comparison across every current operator.
type Input = Scenario;
/// The number of distinct operator variants whose shapes were compared.
type Value = usize;
type Failure = OperatingError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    let populated = match scenario {
        Scenario::Empty => false,
        Scenario::Populated => true,
        Scenario::HostedWithoutSource => return hosted_without_source(),
    };
    let semantic = KnownSyntax;
    let mut symbols = Symbols::default();
    let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText("alpha/beta"));
    let bindings = bindings();
    let mut variants = std::collections::HashSet::new();
    for operation in operations() {
        assert!(variants.insert(std::mem::discriminant(&operation)));
        let tables = tables(&operation, symbol, populated);
        let schemas: Vec<_> = tables
            .iter()
            .map(|table| InputShape {
                kinds: &table.kinds,
                tree: None,
            })
            .collect();
        let actual = schema_of(&operation, &schemas, &bindings);
        let input_rows: Vec<_> = tables.iter().collect();
        let output = fixtures::requests::table_request_context(
            operation.clone(),
            &input_rows,
            bindings.clone(),
            &mut symbols,
            b"",
            Some(&semantic),
        )?;
        assert_eq!(
            actual?, output.kinds,
            "{operation:?}, populated={populated}"
        );
        if matches!(operation, Operation::Hosted { .. }) {
            assert_eq!(output.cells, tables[0].cells, "real request-source rows");
        }
    }
    Ok(variants.len())
}

/// The prepared request has no source partition for the hosted relation.
fn hosted_without_source() -> Result<Value, Failure> {
    let mut symbols = Symbols::default();
    let result = fixtures::requests::published(
        inputs::plan(vec![Operation::Hosted {
            relation: ryo_values::domain::plan_parts::types::HostedNumber(0),
        }]),
        vec![],
        vec![],
        &mut symbols,
    );
    // The prepared request has no source partition for Hosted(0), so admission
    // refuses before schema evaluation. MissingInput belongs to the pure schema
    // contract, independently covered by operation_schema::service_test.
    result.map(|_| 0)
}

/// Both input cardinalities exercise every registered concrete operation fixture.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("all_empty_operator_shapes_match_evaluation"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(26),
    },
    Case {
        name: CaseName("all_populated_operator_shapes_match_evaluation"),
        word: Word::Succeeds,
        input: || Scenario::Populated,
        expect: Expect::Value(26),
    },
    Case {
        name: CaseName("hosted_without_its_request_source_refuses_at_the_real_owner"),
        word: Word::Refuses,
        input: || Scenario::HostedWithoutSource,
        expect: Expect::Error(OperatingError::NotRead),
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
