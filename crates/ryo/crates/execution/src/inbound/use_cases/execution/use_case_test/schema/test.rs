//! Proves every operation schema against its actual empty and populated evaluator.

use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_domain::domain::operation_schema::fixtures::{bindings, operations, tables};
use ryo_execution_domain::domain::operation_schema::service::schema_of;
use ryo_execution_domain::domain::operation_schema::types::InputShape;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::values::KnownSyntax;
use ryo_language::domain::symbols::{
    service::intern_derived,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

/// What a row evaluates.
#[derive(Clone, Copy, Debug)]
enum Evaluation {
    /// Every operator's schema against its evaluator, over empty or populated tables.
    Every { populated: bool },
    /// A hosted relation published without its request source.
    HostedWithoutSource,
}

/// What comparing every operator's schema with its evaluation shows: the operator variants
/// compared, those compared twice, those whose schema is not the kinds evaluation answered, and
/// whether a hosted relation answers its request source's rows.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shapes {
    operators: usize,
    repeated: usize,
    mismatched: Cow<'static, [Cow<'static, str>]>,
    hosted_rows_kept: bool,
}

type Input = Evaluation;
type Value = Shapes;
type Failure = OperatingError;

/// The shapes of every operator: 26 variants, each compared once, every schema matched.
const EVERY_SHAPE_MATCHES: Shapes = Shapes {
    operators: 26,
    repeated: 0,
    mismatched: Cow::Borrowed(&[]),
    hosted_rows_kept: true,
};

/// Evaluates every operator and compares its schema with what it answered.
fn subject(evaluation: Input) -> Result<Value, Failure> {
    let populated = match evaluation {
        Evaluation::Every { populated } => populated,
        Evaluation::HostedWithoutSource => return hosted_without_source(),
    };
    let semantic = KnownSyntax;
    let mut symbols = Symbols::default();
    let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText("alpha/beta"));
    let bindings = bindings();
    let mut variants = std::collections::HashSet::new();
    let mut shapes = Shapes {
        operators: 0,
        repeated: 0,
        mismatched: Cow::Owned(Vec::new()),
        hosted_rows_kept: true,
    };
    for operation in operations() {
        shapes.repeated += usize::from(!variants.insert(std::mem::discriminant(&operation)));
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
        if actual? != output.kinds {
            shapes
                .mismatched
                .to_mut()
                .push(Cow::Owned(format!("{operation:?}")));
        }
        if matches!(operation, Operation::Hosted { .. }) {
            shapes.hosted_rows_kept &= output.cells == tables[0].cells;
        }
    }
    shapes.operators = variants.len();
    Ok(shapes)
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
    result.map(|_| Shapes {
        operators: 1,
        repeated: 0,
        mismatched: Cow::Borrowed(&[]),
        hosted_rows_kept: false,
    })
}

/// Both input cardinalities exercise every registered concrete operation fixture.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("all_empty_operator_shapes_match_evaluation"),
        word: Word::Succeeds,
        input: || Evaluation::Every { populated: false },
        expect: Expect::Value(EVERY_SHAPE_MATCHES),
    },
    Case {
        name: CaseName("all_populated_operator_shapes_match_evaluation"),
        word: Word::Succeeds,
        input: || Evaluation::Every { populated: true },
        expect: Expect::Value(EVERY_SHAPE_MATCHES),
    },
    Case {
        name: CaseName("hosted_without_its_request_source_refuses_at_the_real_owner"),
        word: Word::Refuses,
        input: || Evaluation::HostedWithoutSource,
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
