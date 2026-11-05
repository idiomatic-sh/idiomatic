//! Differential proofs of literal metadata lifetime and invocation-local dynamic ownership.

use crate::application::file_dag::cross_execution::compute;
use crate::domain::matching::types::prepared::value::PreparedMatchers;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{Rows};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{CellKind, Relation, Value};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::{
    Applied, ColumnIndex, Comparison, Function, Input, LiteralText, Operand, ParameterId,
    QueryLiteral, Test,
};
use ryo_values::domain::plans::types::{Operation, Plan};
use std::sync::atomic::Ordering;

/// One authored text predicate with no data-dependent right operand.
fn select(test: Test, pattern: &str) -> Operation {
    Operation::Select {
        from: Input::Relation(Relation::Comment),
        comparisons: vec![Comparison {
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(pattern.into()))),
            test,
            origin: None,
        }],
        kept: Vec::new(),
    }
}

/// Put an operation into a finite compiled plan without evaluating its patterns.
fn plan(operation: Operation) -> Plan {
    let mut plan = crate::application::file_dag::pipeline::service_test::plan();
    plan.steps.truncate(1);
    plan.steps[0].operation = operation;
    plan.emit.step = ryo_values::domain::plan_parts::types::StepId(0);
    plan
}

/// Every call gets fresh symbols and scratch, just as a new rule/file invocation does.
fn evaluate(
    operation: &Operation,
    subject: Option<&str>,
    bindings: &[ParameterBinding],
    prepared: Option<&PreparedMatchers>,
) -> Result<(Table, usize, usize), OperatingError> {
    let mut symbols = Symbols::default();
    let mut input = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    if let Some(subject) = subject {
        let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText(subject));
        push_row(&mut input, &[Value::Symbol(symbol)]);
    }
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: prepared,
        bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut output = Table::default();
    compute(
        operation,
        &[Rows::Computed(&input)],
        &mut context,
        &mut output,
    )?;
    assert!(scratch.observation.local_resources_released);
    let (regexes, globs) = (scratch.observation.regexes, scratch.observation.globs);
    Ok((output, regexes, globs))
}

/// Exact authored literals compile once across fresh invocations, including replacement users.
#[test]
fn literal_matchers_survive_rule_and_file_workspaces() -> Result<(), OperatingError> {
    let regex = select(Test::Matches, r"\s+");
    let glob = select(Test::GlobAny, "*.rs");
    let replace = Operation::Replace {
        from: Input::Relation(Relation::Comment),
        text: ColumnIndex(0),
        pattern: LiteralText(r"\s+".into()),
        replacement: LiteralText("$1".into()),
    };
    let plans = [
        plan(regex.clone()),
        plan(regex.clone()),
        plan(glob.clone()),
        plan(replace.clone()),
    ];
    let prepared = PreparedMatchers::new(&plans);
    assert_eq!(prepared.builds.load(Ordering::Relaxed), 0);
    for _file in 0..3 {
        for (operation, text) in [
            (&regex, "é\u{2003}雪"),
            (&glob, "src/lib.rs"),
            (&replace, "a \t b"),
        ] {
            let actual = evaluate(operation, Some(text), &[], Some(&prepared))?;
            assert_eq!(actual.0, evaluate(operation, Some(text), &[], None)?.0);
            assert_eq!(
                (actual.1, actual.2),
                (0, 0),
                "literal matchers do not enter invocation caches"
            );
        }
    }
    assert_eq!(
        prepared.builds.load(Ordering::Relaxed),
        2,
        "comparison and replacement share one exact regex"
    );
    Ok(())
}

/// Unused, empty, short-circuited and wrong-type paths never initialize an invalid regex.
#[test]
fn invalid_literals_keep_execution_owned_errors_and_laziness() -> Result<(), OperatingError> {
    use crate::domain::matching::service::matches_prepared;
    use crate::domain::table_scratch::types::Patterns;
    use crate::domain::table_texts::types::{Datum, TextView};
    use ryo_values::domain::plan_parts::types::WholeNumber;
    let invalid = select(Test::Matches, "[");
    let invalid_replace = Operation::Replace {
        from: Input::Relation(Relation::Comment),
        text: ColumnIndex(0),
        pattern: LiteralText("[".into()),
        replacement: LiteralText("x".into()),
    };
    let prepared = PreparedMatchers::new(&[plan(invalid.clone()), plan(invalid_replace.clone())]);
    assert_eq!(evaluate(&invalid, None, &[], Some(&prepared))?.0.count.0, 0);
    assert_eq!(
        evaluate(&invalid_replace, None, &[], Some(&prepared))?
            .0
            .count
            .0,
        0
    );
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
    assert_eq!(
        evaluate(&short, Some("actual"), &[], Some(&prepared))?
            .0
            .count
            .0,
        0
    );
    assert_eq!(
        matches_prepared(
            &Datum::Number(WholeNumber(1)),
            &Datum::Text(TextView("[")),
            Some(&prepared),
            &mut Patterns::default()
        )?,
        crate::domain::table_model::types::Verdict::Fails
    );
    assert_eq!(prepared.builds.load(Ordering::Relaxed), 0);
    let baseline = evaluate(&invalid, Some("actual"), &[], None).unwrap_err();
    for _ in 0..2 {
        assert_eq!(
            evaluate(&invalid, Some("actual"), &[], Some(&prepared)).unwrap_err(),
            baseline
        );
    }
    assert_eq!(prepared.builds.load(Ordering::Relaxed), 1);
    assert_eq!(
        evaluate(&invalid_replace, Some("actual"), &[], Some(&prepared)).unwrap_err(),
        evaluate(&invalid_replace, Some("actual"), &[], None).unwrap_err(),
    );
    assert_eq!(prepared.builds.load(Ordering::Relaxed), 1);
    assert_eq!(evaluate(&invalid, None, &[], Some(&prepared))?.0.count.0, 0);
    Ok(())
}

/// Parameters, typed lists and row patterns stay local even if their bytes equal a declared literal.
#[test]
fn bound_lists_and_dynamic_patterns_remain_invocation_owned() -> Result<(), OperatingError> {
    let authored = select(Test::Matches, "a.*");
    let glob_literal = select(Test::GlobAny, "a*");
    let prepared = PreparedMatchers::new(&[plan(authored.clone()), plan(glob_literal.clone())]);
    let mut parameter = authored.clone();
    let Operation::Select { comparisons, .. } = &mut parameter else {
        unreachable!()
    };
    comparisons[0].right = Operand::Parameter(ParameterId(0));
    for (pattern, expected) in [("a.*", 1), ("b.*", 0)] {
        let bindings = [ParameterBinding::Text(LiteralText(pattern.into()))];
        let actual = evaluate(&parameter, Some("alpha"), &bindings, Some(&prepared))?;
        assert_eq!(
            actual.0,
            evaluate(&parameter, Some("alpha"), &bindings, None)?.0
        );
        assert_eq!((actual.0.count.0, actual.1), (expected, 1));
    }
    let mut function = authored.clone();
    let Operation::Select { comparisons, .. } = &mut function else {
        unreachable!()
    };
    comparisons[0].right = Operand::Function(Applied {
        function: Function::Concat,
        arguments: vec![
            Operand::Literal(QueryLiteral::Text(LiteralText("a".into()))),
            Operand::Literal(QueryLiteral::Text(LiteralText(".*".into()))),
        ],
    });
    let actual = evaluate(&function, Some("alpha"), &[], Some(&prepared))?;
    assert_eq!((actual.0.count.0, actual.1), (1, 1));
    let mut dynamic = authored;
    let Operation::Select { comparisons, .. } = &mut dynamic else {
        unreachable!()
    };
    comparisons[0].right = Operand::Column(ColumnIndex(0));
    let actual = evaluate(&dynamic, Some("a.*"), &[], Some(&prepared))?;
    assert_eq!((actual.0.count.0, actual.1), (1, 1));
    let mut lists = glob_literal;
    let Operation::Select { comparisons, .. } = &mut lists else {
        unreachable!()
    };
    comparisons[0].right = Operand::Parameter(ParameterId(0));
    for (patterns, expected) in [(vec!["a*", "α*"], 1), (vec!["b*"], 0), (Vec::new(), 0)] {
        let bindings = [ParameterBinding::Texts(
            patterns
                .into_iter()
                .map(|text| LiteralText(text.into()))
                .collect(),
        )];
        let actual = evaluate(&lists, Some("alpha"), &bindings, Some(&prepared))?;
        assert_eq!(
            actual.0,
            evaluate(&lists, Some("alpha"), &bindings, None)?.0
        );
        assert_eq!((actual.0.count.0, actual.2), (expected, 1));
    }
    assert_eq!(prepared.builds.load(Ordering::Relaxed), 0);
    Ok(())
}

/// Test family/polarity, exact bytes, Unicode interpreter fallback and separate requests never alias.
#[test]
fn literal_glob_fallback_and_family_identity_match_local_execution() -> Result<(), OperatingError> {
    let operations = [
        select(Test::GlobAny, "*"),
        select(Test::NotGlobAny, "*"),
        select(Test::GlobAny, "α?"),
        select(Test::Matches, "*"),
    ];
    let prepared = PreparedMatchers::new(&operations.iter().cloned().map(plan).collect::<Vec<_>>());
    for _ in 0..2 {
        for operation in &operations[..3] {
            let actual = evaluate(operation, Some("αβ"), &[], Some(&prepared))?;
            assert_eq!(actual.0, evaluate(operation, Some("αβ"), &[], None)?.0);
            assert_eq!((actual.1, actual.2), (0, 0));
        }
    }
    assert_eq!(
        prepared.builds.load(Ordering::Relaxed),
        3,
        "interpreted glob outcome also persists"
    );
    assert!(evaluate(&operations[3], Some("αβ"), &[], Some(&prepared)).is_err());
    assert_eq!(prepared.builds.load(Ordering::Relaxed), 4);
    let another_request = PreparedMatchers::new(&[plan(select(Test::GlobAny, "b*"))]);
    let actual = evaluate(&operations[0], Some("αβ"), &[], Some(&another_request))?;
    assert_eq!(
        (actual.0.count.0, actual.2),
        (1, 1),
        "an undeclared literal keeps its ordinary local path"
    );
    assert_eq!(another_request.builds.load(Ordering::Relaxed), 0);
    Ok(())
}

/// Concurrent users of one immutable request cannot race two initial compilations.
#[test]
fn concurrent_literal_users_initialize_one_slot() {
    let prepared = PreparedMatchers::new(&[plan(select(Test::Matches, "α+"))]);
    std::thread::scope(|scope| {
        let prepared = &prepared;
        let handles: Vec<_> = (0..4)
            .map(|_| scope.spawn(move || prepared.regex("α+").unwrap().unwrap().is_match("αα")))
            .collect();
        for handle in handles {
            assert!(handle.join().unwrap());
        }
    });
    assert_eq!(prepared.builds.load(Ordering::Relaxed), 1);
}

use ryo_values::domain::rows::types::Table;

use ryo_values::domain::query_bindings::types::{ParameterBinding};
