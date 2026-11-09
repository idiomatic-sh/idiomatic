//! The shared walk keeps errors in their authored order: predicates short-circuit as written, a memoised predicate builds its error
//! once, and the first source row owns the first error.
use super::{evaluate_plans, plan, plans};
use crate::domain::operator::types::OperatorContext;
use crate::domain::shared_walk::types::WalkWork;
use crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk;
use crate::domain::step_classes::types::ClassPlace;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{FileId, KindBits, Offset, Relation};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::plans::types::{Operation, Plan};

/// A plan with an invalid regular expression before or after its name test.
fn with_invalid(name: &str, first: bool) -> Plan {
    let mut each = plan(name);
    let Operation::Select { comparisons, .. } = &mut each.steps[0].operation else {
        panic!("the fixture builds a Select")
    };
    let at = if first { 0 } else { comparisons.len() };
    comparisons.insert(
        at,
        PlanComparison {
            test: Test::Matches,
            left: Operand::Column(ColumnIndex(2)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText("[".into()))),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        },
    );
    each
}

fn predicate_order_preserves_errors_and_short_circuits() {
    let plans = [
        with_invalid("absent-kind", true),
        with_invalid("absent-kind", false),
    ];
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let batch = evaluate_plans(&prepared, &bindings, &[ClassPlace(0), ClassPlace(1)]);
    assert!(matches!(
        batch.class(ClassPlace(0)).map(|result| &result.completion),
        Some(Err(
            crate::domain::operating::errors::OperatingError::InvalidRegex { .. }
        ))
    ));
    assert!(batch.class(ClassPlace(1)).unwrap().positions.0.is_empty());
}

fn memo_builds_an_erroring_predicate_once_and_reuses_hits() {
    let plans = [with_invalid("word", true), with_invalid("other", true)];
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let batch = evaluate_plans(&prepared, &bindings, &[ClassPlace(0), ClassPlace(1)]);
    assert!(
        [ClassPlace(0), ClassPlace(1)]
            .into_iter()
            .all(|class| matches!(
                batch.class(class).map(|result| &result.completion),
                Some(Err(
                    crate::domain::operating::errors::OperatingError::InvalidRegex { .. }
                ))
            ))
    );
    assert_eq!(batch.counts().predicate_tests.0, 1);
    assert_eq!(batch.counts().predicate_reuses.0, 1);
}

fn different_kind_errors_keep_the_first_source_row() {
    let source = b"[(a";
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(source));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(3), Offset(0), Offset(1));
    token(&mut builder, KindBits(2), Offset(1), Offset(2));
    token(&mut builder, KindBits(2), Offset(2), Offset(3));
    leave_node(&mut builder, Offset(3));
    for (kind, name) in [(1, "file"), (2, "second"), (3, "first")] {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(source), FileId(0)),
            KindBits(kind),
            NameText(name),
        );
    }
    index_kinds(&mut elements);
    let mut plans = plans();
    for rule in [0, 2] {
        let Operation::Select { comparisons, .. } = &mut plans[rule].steps[0].operation else {
            panic!("the fixture builds a Select");
        };
        *comparisons = vec![
            PlanComparison {
                test: Test::NotEqual,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("file".into()))),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            },
            PlanComparison {
                test: Test::Matches,
                left: Operand::Literal(QueryLiteral::Text(LiteralText("x".into()))),
                right: Operand::Column(ColumnIndex(2)),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            },
        ];
    }
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: crate::domain::matching::service_test::matcher(),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let bound = bindings
        .iter()
        .cloned()
        .map(Vec::into_boxed_slice)
        .collect::<Vec<_>>();
    let batch = super::evaluate_rows(
        &prepared,
        Relation::SyntaxTree,
        &crate::domain::table_model::types::Rows::Tree(TreeRows {
            elements: &elements,
            file: FileId(0),
            form: TreeForm::Elements,
        }),
        &bound,
        &mut context,
        &[ClassPlace(0), ClassPlace(1), ClassPlace(2)],
        &mut WalkWork::default(),
    );
    let Some(Err(crate::domain::operating::errors::OperatingError::InvalidRegex { reason })) =
        batch.class(ClassPlace(0)).map(|result| &result.completion)
    else {
        panic!("first source row must retain its regex error");
    };
    assert!(reason.contains('['), "{reason}");
    assert_eq!(batch.visits(ClassPlace(0)).0, 1);
}

#[derive(Clone, Copy)]
enum Scenario {
    PredicateOrder,
    SourceOrder,
    MemoReuse,
}

type Input = Scenario;
type Value = usize;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("predicates preserve authored error and short-circuit order"),
        word: Word::Succeeds,
        input: || Scenario::PredicateOrder,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("memoised predicates reuse hits and build an error once"),
        word: Word::Succeeds,
        input: || Scenario::MemoReuse,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("first source row owns first error"),
        word: Word::Succeeds,
        input: || Scenario::SourceOrder,
        expect: Expect::Value(1),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::PredicateOrder => predicate_order_preserves_errors_and_short_circuits(),
        Scenario::SourceOrder => different_kind_errors_keep_the_first_source_row(),
        Scenario::MemoReuse => memo_builds_an_erroring_predicate_once_and_reuses_hits(),
    }
    Ok(1)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
