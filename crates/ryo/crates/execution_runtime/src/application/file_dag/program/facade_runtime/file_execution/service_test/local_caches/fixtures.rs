//! The support of the `local_caches` cases.
use super::super::super::*;
use super::super::ordered_dfs::fixtures::execute_bound_request;
use crate::application::file_dag::fixtures::operator_contract::inputs::table_inputs;
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::symbols::{service::intern_derived, types::ByteSlice};
use ryo_relations::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word};
use ryo_values::domain::plan_parts::types::{
    Applied, Function, LiteralText, Operand, ParameterId, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::Table,
};

pub(crate) type Value = ();

pub(crate) type Failure = OperatingError;

pub(crate) type Input = fn() -> Result<Value, Failure>;

pub(super) const FROM: PlanInput = PlanInput::Relation(Relation::Function);

pub(super) fn select(test: Test, pattern: &str) -> Operation {
    Operation::Select {
        from: FROM,
        kept: vec![],
        comparisons: vec![PlanComparison {
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(pattern.into()))),
            test,
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        }],
    }
}

/// The regexes and globs an invocation compiled into its local caches, with the answer's row count: one text row, the
/// operation, and an undemanded literal `shadow` step whose compiled slot a dynamic pattern must not take by equal bytes.
pub(super) fn local_caches(
    operation: Operation,
    text: &str,
    bindings: &[ParameterBinding],
    shadow: &Operation,
) -> Result<(usize, usize, usize), OperatingError> {
    let mut symbols = Symbols::default();
    let mut input = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText(text));
    push_row(&mut input, &[Cell::Symbol(symbol)]);
    let mut authored = table_inputs(operation, &[&input], bindings, &mut symbols);
    let mut shadow = shadow.clone();
    ryo_values::domain::plans::service::each_input(&mut shadow, &mut |input| {
        *input = PlanInput::Step(StepId(0));
    });
    authored.plan.steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: shadow,
    });
    let mut observed = ExecutionObservation::default();
    let out = execute_bound_request(
        authored.plan,
        bindings.to_vec(),
        &authored.hosted,
        &authored.facts,
        &mut symbols,
        b"",
        false,
        &mut observed,
    )?;
    Ok((out.count.0, observed.regexes, observed.globs))
}

/// Finite literal patterns are prepared once for the program and never enter an invocation's local caches; dynamic
/// patterns (bound, computed or read from a row) do, once.
pub(super) fn only_dynamic_patterns_enter_local_caches() -> Result<Value, Failure> {
    for (operation, text) in [
        (select(Test::Matches, r"\s+"), "é\u{2003}雪"),
        (select(Test::GlobAny, "*.rs"), "src/lib.rs"),
        (select(Test::GlobAny, "*"), "αβ"),
        (select(Test::NotGlobAny, "*"), "αβ"),
        (select(Test::GlobAny, "α?"), "αβ"),
        (
            Operation::Replace {
                from: FROM,
                text: ColumnIndex(0),
                pattern: LiteralText(r"\s+".into()),
                replacement: LiteralText("$1".into()),
            },
            "a \t b",
        ),
    ] {
        let (_, regexes, globs) = local_caches(operation.clone(), text, &[], &operation)?;
        assert_eq!(
            (regexes, globs),
            (0, 0),
            "finite literal slots never enter local invocation caches"
        );
    }
    let shadow = select(Test::Matches, "a.*");
    let mut parameter = shadow.clone();
    if let Operation::Select { comparisons, .. } = &mut parameter {
        comparisons[0].right = Operand::Parameter(ParameterId(0));
    }
    for (pattern, expected) in [("a.*", 1), ("b.*", 0)] {
        let bindings = [ParameterBinding::Text(LiteralText(pattern.into()))];
        let (count, regexes, _) = local_caches(parameter.clone(), "alpha", &bindings, &shadow)?;
        assert_eq!(
            (count, regexes),
            (expected, 1),
            "a bound value cannot reuse the same-byte finite slot"
        );
    }
    let mut function = shadow.clone();
    if let Operation::Select { comparisons, .. } = &mut function {
        comparisons[0].right = Operand::Function(Applied {
            function: Function::Concat,
            arguments: vec![
                Operand::Literal(QueryLiteral::Text(LiteralText("a".into()))),
                Operand::Literal(QueryLiteral::Text(LiteralText(".*".into()))),
            ],
        });
    }
    let mut row = shadow.clone();
    if let Operation::Select { comparisons, .. } = &mut row {
        comparisons[0].right = Operand::Column(ColumnIndex(0));
    }
    for (operation, text) in [(function, "alpha"), (row, "a.*")] {
        let (count, regexes, _) = local_caches(operation, text, &[], &shadow)?;
        assert_eq!((count, regexes), (1, 1));
    }
    let shadow = select(Test::GlobAny, "a*");
    let mut lists = shadow.clone();
    if let Operation::Select { comparisons, .. } = &mut lists {
        comparisons[0].right = Operand::Parameter(ParameterId(0));
    }
    for (patterns, expected) in [(vec!["a*", "α*"], 1), (vec!["b*"], 0), (vec![], 0)] {
        let bindings = [ParameterBinding::Texts(
            patterns
                .into_iter()
                .map(|text| LiteralText(text.into()))
                .collect(),
        )];
        let (count, _, globs) = local_caches(lists.clone(), "alpha", &bindings, &shadow)?;
        assert_eq!((count, globs), (expected, 1));
    }
    Ok(())
}

pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(stringify!(only_dynamic_patterns_enter_local_caches)),
    word: Word::Succeeds,
    input: || only_dynamic_patterns_enter_local_caches,
    expect: Expect::Value(()),
}];

pub(crate) fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
