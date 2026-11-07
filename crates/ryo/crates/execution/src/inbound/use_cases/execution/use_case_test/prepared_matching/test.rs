//! Literal metadata and dynamic patterns are exercised through real prepared file owners.
use crate::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use super::super::super::{fixtures, use_case::ExecutionUseCaseImpl};
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::domain::operating::errors::OperatingError;
use ryo_language::domain::{
    elements::types::Elements,
    symbols::{
        service::intern_derived,
        types::{ByteSlice, Symbols},
    },
};
use ryo_relations::domain::{
    relations::types::{CellKind, Relation, Value as Cell},
    terms::types::NameText,
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{
            Applied, ColumnIndex, PlanComparison, Function, Input as Source, LiteralText, Operand,
            ParameterId, QueryLiteral, StepId, StepScope, Test,
        },
        plans::types::{Operation, PlanStep},
        query_bindings::types::ParameterBinding,
        rows::{service::push_row, types::Table},
    },
};
use std::sync::atomic::Ordering;
#[derive(Clone, Copy)]
enum Scenario {
    LiteralLifetime,
    InvalidLazy,
    Dynamic,
    GlobFamilies,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
fn select(test: Test, pattern: &str) -> Operation {
    Operation::Select {
        from: Source::Relation(Relation::Function),
        kept: vec![],
        comparisons: vec![PlanComparison {
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(pattern.into()))),
            test,
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        }],
    }
}
fn prepare(
    operation: Operation,
    subject: Option<&str>,
    bindings: &[ParameterBinding],
    shadow: &Operation,
) -> Result<
    (
        PrepareUseCaseImpl,
        PreparedExecutionProgram,
        Symbols,
        Table,
    ),
    Failure,
> {
    let mut symbols = Symbols::default();
    let mut input = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    if let Some(text) = subject {
        let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText(text));
        push_row(&mut input, &[Cell::Symbol(symbol)]);
    }
    let mut authored = fixtures::table_inputs(operation, &[&input], bindings, &mut symbols);
    // A real but undemanded authored literal remains in the same immutable request.
    // Dynamic callers must not acquire its already-compiled slot merely by equal bytes.
    let mut shadow = shadow.clone();
    ryo_values::domain::plans::service::each_input(&mut shadow, &mut |input| {
        *input = Source::Step(StepId(0))
    });
    authored.plan.steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: shadow,
    });
    let (engine, program) = fixtures::prepared(authored.plan, bindings.to_vec(), authored.hosted)?;
    Ok((engine, program, symbols, input))
}
fn evaluate(
    engine: &PrepareUseCaseImpl,
    program: &PreparedExecutionProgram,
    symbols: &mut Symbols,
) -> Result<(Table, usize, usize), Failure> {
    let read = |_| Err(OperatingError::NotRead);
    let (table, observed) =
        fixtures::run_prepared(engine, program, b"", &Elements::default(), symbols, &read)?;
    assert!(
        observed.file_owner_released,
        "actual file-owner weak handle is dead after finish, so mandatory workspace references were released"
    );
    Ok((table, observed.regexes, observed.globs))
}
fn builds(program: &PreparedExecutionProgram) -> usize {
    program.inner.matchers().builds.load(Ordering::Relaxed)
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::LiteralLifetime => {
            for (operation, text, replaced) in [
                (select(Test::Matches, r"\s+"), "é\u{2003}雪", false),
                (select(Test::GlobAny, "*.rs"), "src/lib.rs", false),
                (
                    Operation::Replace {
                        from: Source::Relation(Relation::Function),
                        text: ColumnIndex(0),
                        pattern: LiteralText(r"\s+".into()),
                        replacement: LiteralText("$1".into()),
                    },
                    "a \t b",
                    true,
                ),
            ] {
                let (engine, program, mut symbols, input) =
                    prepare(operation.clone(), Some(text), &[], &operation)?;
                assert_eq!(builds(&program), 0);
                for _file in 0..3 {
                    let actual = evaluate(&engine, &program, &mut symbols)?;
                    assert_eq!(
                        (actual.1, actual.2),
                        (0, 0),
                        "finite literal slots never enter local invocation caches"
                    );
                    if replaced {
                        // Replace appends one literal replacement cell and keeps its input.
                        let value = intern_derived(&mut symbols, ByteSlice(b""), NameText("a$1b"));
                        assert_eq!(actual.0.count, input.count);
                        assert_eq!(actual.0.kinds, [CellKind::Symbol, CellKind::Symbol]);
                        assert_eq!(actual.0.cells, [input.cells[0], Cell::Symbol(value)]);
                    } else {
                        assert_eq!(actual.0, input);
                    }
                }
                assert_eq!(
                    builds(&program),
                    1,
                    "one literal compilation survives fresh file owners"
                );
            }
        }
        Scenario::InvalidLazy => {
            let invalid = select(Test::Matches, "[");
            let replace = Operation::Replace {
                from: Source::Relation(Relation::Function),
                text: ColumnIndex(0),
                pattern: LiteralText("[".into()),
                replacement: LiteralText("x".into()),
            };
            for operation in [&invalid, &replace] {
                let (engine, program, mut symbols, _) =
                    prepare(operation.clone(), None, &[], operation)?;
                assert_eq!(evaluate(&engine, &program, &mut symbols)?.0.count.0, 0);
                assert_eq!(builds(&program), 0);
                let (engine, program, mut symbols, _) =
                    prepare(operation.clone(), Some("actual"), &[], operation)?;
                let expected = crate::domain::matching::types::prepared::value::compile_regex("[")
                    .unwrap_err();
                for _file in 0..2 {
                    assert_eq!(
                        evaluate(&engine, &program, &mut symbols).unwrap_err(),
                        expected
                    );
                }
                assert_eq!(builds(&program), 1);
            }
            let mut short = select(Test::Equal, "never");
            if let (
                Operation::Select { comparisons, .. },
                Operation::Select {
                    comparisons: tail, ..
                },
            ) = (&mut short, &invalid)
            {
                comparisons.extend(tail.clone());
            }
            let (engine, program, mut symbols, _) = prepare(short, Some("actual"), &[], &invalid)?;
            assert_eq!(evaluate(&engine, &program, &mut symbols)?.0.count.0, 0);
            assert_eq!(
                builds(&program),
                0,
                "false earlier predicate never initializes invalid regex"
            );
        }
        Scenario::Dynamic => {
            let shadow = select(Test::Matches, "a.*");
            let mut parameter = shadow.clone();
            if let Operation::Select { comparisons, .. } = &mut parameter {
                comparisons[0].right = Operand::Parameter(ParameterId(0));
            }
            for (pattern, expected) in [("a.*", 1), ("b.*", 0)] {
                let bindings = [ParameterBinding::Text(LiteralText(pattern.into()))];
                let (engine, program, mut symbols, _) =
                    prepare(parameter.clone(), Some("alpha"), &bindings, &shadow)?;
                assert!(program.inner.matchers().regex("a.*").unwrap().is_ok());
                let actual = evaluate(&engine, &program, &mut symbols)?;
                assert_eq!((actual.0.count.0, actual.1), (expected, 1));
                assert_eq!(
                    builds(&program),
                    1,
                    "dynamic bound value cannot reuse same-byte finite slot"
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
                let (engine, program, mut symbols, _) =
                    prepare(operation, Some(text), &[], &shadow)?;
                assert!(program.inner.matchers().regex("a.*").unwrap().is_ok());
                let actual = evaluate(&engine, &program, &mut symbols)?;
                assert_eq!((actual.0.count.0, actual.1), (1, 1));
                assert_eq!(builds(&program), 1);
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
                let (engine, program, mut symbols, _) =
                    prepare(lists.clone(), Some("alpha"), &bindings, &shadow)?;
                assert!(program.inner.matchers().glob(Test::GlobAny, "a*").is_some());
                let actual = evaluate(&engine, &program, &mut symbols)?;
                assert_eq!((actual.0.count.0, actual.2), (expected, 1));
                assert_eq!(builds(&program), 1);
            }
        }
        Scenario::GlobFamilies => {
            for (test, pattern, expected) in [
                (Test::GlobAny, "*", 1),
                (Test::NotGlobAny, "*", 0),
                (Test::GlobAny, "α?", 1),
            ] {
                let operation = select(test, pattern);
                let (engine, program, mut symbols, _) =
                    prepare(operation.clone(), Some("αβ"), &[], &operation)?;
                for _file in 0..2 {
                    let actual = evaluate(&engine, &program, &mut symbols)?;
                    assert_eq!((actual.0.count.0, actual.1, actual.2), (expected, 0, 0));
                }
                assert_eq!(builds(&program), 1);
            }
            let invalid = select(Test::Matches, "*");
            let (engine, program, mut symbols, _) =
                prepare(invalid.clone(), Some("αβ"), &[], &invalid)?;
            assert!(matches!(
                evaluate(&engine, &program, &mut symbols),
                Err(OperatingError::InvalidRegex { .. })
            ));
            assert_eq!(builds(&program), 1);
        }
    }
    Ok(())
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("literal_matchers_survive_real_rule_and_file_workspaces"),
        word: Word::Succeeds,
        input: || Scenario::LiteralLifetime,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("invalid_literals_keep_errors_empty_and_short_circuit_laziness"),
        word: Word::Succeeds,
        input: || Scenario::InvalidLazy,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("bound_lists_functions_and_row_patterns_remain_invocation_owned"),
        word: Word::Succeeds,
        input: || Scenario::Dynamic,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("literal_glob_fallback_family_and_polarity_match_real_output"),
        word: Word::Succeeds,
        input: || Scenario::GlobFamilies,
        expect: Expect::Value(()),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
