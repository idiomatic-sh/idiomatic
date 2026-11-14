//! What the bound predicates acquire and short circuit: a failed preceding comparison never
//! acquires the invalid regex after it, and a passing one acquires it once and is refused.
use crate::application::file_dag::predicates::constants::{
    EMPTY_SOURCE, INVALID_PATTERN, PASS, STOP, SUBJECT,
};
use crate::application::file_dag::predicates::types::BoundPredicate;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operator::contract::types::PreparedPredicates;
use ryo_execution_domain::domain::operator::contract::views::types::PredicateProgram;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers;
use ryo_relations::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as PlanInput, LiteralText, Operand, PlanComparison,
    QueryLiteral, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use std::borrow::Cow;
use std::sync::atomic::Ordering;

/// The verdict on the row's text, and how many regexes the prepared matchers built for it.
#[derive(Debug, PartialEq)]
struct Held {
    verdict: PredicateOutcome,
    builds: usize,
}

/// The refusal of the row's text, in full, and how many regexes the prepared matchers built.
#[derive(Debug, PartialEq)]
struct Refused {
    error: Cow<'static, str>,
    builds: usize,
}

/// The text of the one row the predicates judge: equal to `pass`, then matching the invalid
/// pattern.
type Input = &'static str;
type Value = Held;
type Failure = Refused;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("failed_preceding_predicate_never_acquires_invalid_regex"),
        word: Word::Succeeds,
        input: || STOP,
        expect: Expect::Value(Held {
            verdict: PredicateOutcome::Fails,
            builds: 0,
        }),
    },
    Case {
        name: CaseName("a_passing_preceding_predicate_acquires_the_invalid_regex_once"),
        word: Word::Refuses,
        input: || PASS,
        expect: Expect::Error(Refused {
            error: Cow::Borrowed(
                "InvalidRegex { reason: \"regex parse error:\\n    [\\n    ^\\nerror: unclosed character class\" }",
            ),
            builds: 1,
        }),
    },
];

/// The two comparisons: the row's text equals `pass`, then the subject matches the invalid
/// pattern.
fn comparisons() -> Vec<PlanComparison> {
    let text = |text: &str| QueryLiteral::Text(LiteralText(text.into()));
    vec![
        PlanComparison {
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Literal(text(PASS)),
            test: Test::Equal,
            origin: ComparisonOrigin::Unrecorded,
        },
        PlanComparison {
            left: Operand::Literal(text(SUBJECT)),
            right: Operand::Literal(text(INVALID_PATTERN)),
            test: Test::Matches,
            origin: ComparisonOrigin::Unrecorded,
        },
    ]
}

/// Prepares the select of the two comparisons and judges the one row of `text`.
fn subject(text: Input) -> Result<Value, Failure> {
    let comparisons = comparisons();
    let plan = Plan {
        parameters: vec![],
        observation_inputs: vec![],
        observations: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::Comment),
                comparisons: comparisons.clone(),
                kept: vec![],
            },
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    };
    let prepared = PreparedMatchers::new(&[plan]);
    let program = PreparedPredicates {
        comparisons,
        kinds: vec![CellKind::Symbol],
    };
    let mut symbols = Symbols::default();
    let row = intern_derived(&mut symbols, ByteSlice(EMPTY_SOURCE), NameText(text));
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
    let held = evaluator.holds(&[Cell::Symbol(row)], &symbols);
    let builds = prepared.builds.load(Ordering::Relaxed);
    held.map(|verdict| Held { verdict, builds })
        .map_err(|error| Refused {
            error: Cow::Owned(format!("{error:?}")),
            builds,
        })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
