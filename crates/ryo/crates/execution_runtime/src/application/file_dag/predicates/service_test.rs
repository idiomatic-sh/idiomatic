//! What the bound predicates acquire and short circuit.
use super::types::BoundPredicate;
use ryo_execution_domain::domain::matching::types::prepared::value::PreparedMatchers;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::types::PreparedPredicates;
use ryo_execution_domain::domain::operator::contract::views::types::PredicateProgram;
use ryo_execution_domain::domain::table_model::types::Verdict;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    Input, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::plans::types::Operation;
use std::sync::atomic::Ordering;

use super::constants::{EMPTY_SOURCE, INVALID_PATTERN, PASS, STOP, SUBJECT};

#[test]
fn failed_preceding_predicate_never_acquires_invalid_regex() {
    let comparisons = vec![
        PlanComparison {
            left: Operand::Column(ryo_values::domain::plan_parts::types::ColumnIndex(0)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(PASS.into()))),
            test: Test::Equal,
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        },
        PlanComparison {
            left: Operand::Literal(QueryLiteral::Text(LiteralText(SUBJECT.into()))),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(INVALID_PATTERN.into()))),
            test: Test::Matches,
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        },
    ];
    let operation = Operation::Select {
        from: Input::Relation(Relation::Comment),
        comparisons: comparisons.clone(),
        kept: vec![],
    };
    let plan = ryo_values::domain::plans::types::Plan {
        parameters: vec![],
        observation_inputs: vec![],
        observations: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        steps: vec![ryo_values::domain::plans::types::PlanStep {
            scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
            operation,
        }],
        emit: ryo_values::domain::plans::types::Emit {
            step: ryo_values::domain::plan_parts::types::StepId(0),
            subject: ryo_values::domain::plan_parts::types::ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    };
    let prepared = PreparedMatchers::new(&[plan]);
    let program = PreparedPredicates {
        comparisons,
        kinds: vec![ryo_relations::domain::relations::types::CellKind::Symbol],
    };
    let mut symbols = Symbols::default();
    let stop = ryo_language::domain::symbols::service::intern_derived(
        &mut symbols,
        ryo_language::domain::symbols::types::ByteSlice(EMPTY_SOURCE),
        ryo_relations::domain::terms::types::NameText(STOP),
    );
    let pass = ryo_language::domain::symbols::service::intern_derived(
        &mut symbols,
        ryo_language::domain::symbols::types::ByteSlice(EMPTY_SOURCE),
        ryo_relations::domain::terms::types::NameText(PASS),
    );
    let mut state = Default::default();
    let mut memo = Vec::new();
    let mut names = None;
    #[cfg(feature = "diagnostic-row-progress")]
    let routes =
        ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile::default(
        );
    let mut evaluator = BoundPredicate {
        program: &program,
        bindings: &[],
        source: SourceView(EMPTY_SOURCE),
        paths: &[],
        arenas: &[],
        prepared: Some(&prepared),
        state: &mut state,
        memo: &mut memo,
        names: &mut names,
        #[cfg(feature = "diagnostic-row-progress")]
        progress: &routes,
    };
    assert_eq!(
        evaluator.holds(&[Value::Symbol(stop)], &symbols).unwrap(),
        Verdict::Fails
    );
    assert_eq!(prepared.builds.load(Ordering::Relaxed), 0);
    assert!(matches!(
        evaluator.holds(&[Value::Symbol(pass)], &symbols),
        Err(OperatingError::InvalidRegex { .. })
    ));
    assert_eq!(prepared.builds.load(Ordering::Relaxed), 1);
}
