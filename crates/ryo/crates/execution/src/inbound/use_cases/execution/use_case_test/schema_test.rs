//! Proves every operation schema against its actual empty and populated evaluator.

use super::super::fixtures;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::KnownSyntax;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::fixtures::{bindings, operations, tables};
use crate::domain::operation_schema::service::schema_of;
use crate::domain::operation_schema::types::InputShape;
use ryo_language::domain::symbols::{
    service::intern_derived,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plans::types::Operation;

use ryo_relations::domain::relations::types::Relation;
/// A genuine populated/empty evaluator comparison across every current operator.
type Input = bool;
/// The number of distinct operator variants whose shapes were compared.
type Value = usize;
type Failure = OperatingError;

fn subject(populated: Input) -> Result<Value, Failure> {
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
        let (output, _) = fixtures::table_request_context(
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

/// Both input cardinalities exercise every registered concrete operation fixture.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("all_empty_operator_shapes_match_evaluation"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(26),
    },
    Case {
        name: CaseName("all_populated_operator_shapes_match_evaluation"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(26),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn hosted_without_its_request_source_refuses_at_the_real_owner() {
    let mut symbols = Symbols::default();
    let result = fixtures::published(
        fixtures::plan(vec![Operation::Hosted {
            relation: ryo_values::domain::plan_parts::types::HostedNumber(0),
        }]),
        vec![],
        vec![],
        &mut symbols,
    );
    assert_eq!(result, Err(OperatingError::MissingInput { position: 1 }));
}
