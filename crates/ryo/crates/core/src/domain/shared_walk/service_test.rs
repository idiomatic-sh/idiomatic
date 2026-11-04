//! The prepared native pass keeps independent class results and never publishes.

use crate::domain::{
    coverage::types::RuleIndex,
    operating::types::OperatorContext,
    positions::{
        service_test::{SMALL_SOURCE, SmallTree, small_tree},
        types::ClassPlace,
    },
    shared_walk::{
        service::evaluate,
        types::{NativeBatch, PreparedWalk, WalkWork},
    },
    table_cells::types::{TreeForm, TreeRows},
    table_model::types::ParameterBinding,
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use ryo_language::domain::{
    completion::service::{completion, name_kind},
    elements::service::{enter_node, leave_node, table_builder, token},
    elements::types::Elements,
    kind_index::service::index_kinds,
    symbols::types::{ByteSlice, Symbols},
};
use ryo_relations::domain::{
    relations::types::{FileId, KindBits, Offset, Relation},
    terms::types::NameText,
};
use ryo_values::domain::{
    plan_parts::types::{
        ColumnIndex, Comparison, Input, LiteralText, Operand, ParameterId, QueryLiteral, StepId,
        Test,
    },
    plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
};

fn plan(name: &str) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(if name.is_empty() { 2 } else { 1 })),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText(name.into()))),
                    origin: None,
                }],
                kept: Vec::new(),
            },
            scope: StepScope::PerFile,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

fn plans() -> [Plan; 4] {
    [plan("word"), plan("other"), plan("word"), plan("")]
}

fn evaluate_plans(
    prepared: &PreparedWalk,
    bindings: &[Vec<ParameterBinding>],
    active: &[bool],
) -> NativeBatch {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let tree = TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Elements,
    };
    let bound = bindings
        .iter()
        .cloned()
        .map(Vec::into_boxed_slice)
        .collect::<Vec<_>>();
    let mut operator_scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut operator_scratch,
    };
    evaluate(
        prepared,
        tree,
        &bound,
        &mut context,
        active,
        &mut WalkWork::default(),
    )
}

#[test]
fn equivalent_rules_share_one_ordered_class_without_retaining_plans() {
    let plans = plans();
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    assert_eq!(prepared.classes().len(), 3);
    assert_eq!(prepared.classes()[0].members.len(), 2);
    assert_eq!(
        prepared.class_of(RuleIndex(2)).unwrap()[0],
        Some(ClassPlace(0))
    );
    let batch = evaluate_plans(&prepared, &bindings, &[true; 4]);
    assert_eq!(
        batch.class(ClassPlace(0)).unwrap().as_ref().unwrap().0,
        [1, 3]
    );
    assert_eq!(batch.visits(ClassPlace(0)), 2);
    assert_eq!(batch.visits(ClassPlace(1)), 0);
    assert!(batch.class(ClassPlace(1)).is_none());
    assert!(batch.class(ClassPlace(2)).is_none());
}

#[test]
fn only_referenced_bindings_affect_class_equivalence() {
    let plans = plans();
    let mut bindings = vec![Vec::new(); plans.len()];
    bindings[0] = vec![ParameterBinding::Text(LiteralText("[".into()))];
    bindings[2] = vec![
        ParameterBinding::Text(LiteralText("[".into())),
        ParameterBinding::Numbers(Vec::new()),
    ];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    assert_eq!(prepared.classes()[0].members.len(), 2);
    assert_eq!(
        evaluate_plans(&prepared, &bindings, &[true; 4])
            .class(ClassPlace(0))
            .unwrap()
            .as_ref()
            .unwrap()
            .0,
        [1, 3]
    );
}

#[test]
fn inactive_representative_and_single_member_preserve_eligibility() {
    let mut plans = plans();
    plans[1] = plan("word");
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let with_two = evaluate_plans(&prepared, &bindings, &[false, true, true, false]);
    assert_eq!(
        with_two.class(ClassPlace(0)).unwrap().as_ref().unwrap().0,
        [1, 3]
    );
    let with_one = evaluate_plans(&prepared, &bindings, &[true, false, false, false]);
    assert!(with_one.into_classes().iter().all(Option::is_none));
}

fn add_pattern(plans: &mut [Plan; 4], parameter: u32) {
    for rule in [0, 2] {
        let Operation::Select { comparisons, .. } = &mut plans[rule].steps[0].operation else {
            unreachable!();
        };
        comparisons.push(Comparison {
            test: Test::Matches,
            left: Operand::Column(ColumnIndex(2)),
            right: Operand::Parameter(ParameterId(parameter)),
            origin: None,
        });
    }
}

#[test]
fn a_reached_semantic_error_is_kept_for_its_class() {
    let mut plans = plans();
    add_pattern(&mut plans, 0);
    let mut bindings = vec![Vec::new(); plans.len()];
    bindings[0] = vec![ParameterBinding::Text(LiteralText("[".into()))];
    bindings[2] = vec![ParameterBinding::Text(LiteralText("[".into()))];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    assert_eq!(prepared.classes()[0].members.len(), 2);
    let batch = evaluate_plans(&prepared, &bindings, &[true; 4]);
    assert!(matches!(batch.class(ClassPlace(0)), Some(Err(_))));
    assert_eq!(batch.visits(ClassPlace(0)), 1);
    assert!(batch.class(ClassPlace(1)).is_none());
}

#[test]
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
            unreachable!();
        };
        *comparisons = vec![
            Comparison {
                test: Test::NotEqual,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("file".into()))),
                origin: None,
            },
            Comparison {
                test: Test::Matches,
                left: Operand::Literal(QueryLiteral::Text(LiteralText("x".into()))),
                right: Operand::Column(ColumnIndex(2)),
                origin: None,
            },
        ];
    }
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
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
    let batch = evaluate(
        &prepared,
        TreeRows {
            elements: &elements,
            file: FileId(0),
            form: TreeForm::Elements,
        },
        &bound,
        &mut context,
        &[true; 4],
        &mut WalkWork::default(),
    );
    let Some(Err(crate::domain::operating::errors::OperatingError::InvalidRegex { reason })) =
        batch.class(ClassPlace(0))
    else {
        panic!("first source row must retain its regex error");
    };
    assert!(reason.contains('['), "{reason}");
    assert_eq!(batch.visits(ClassPlace(0)), 1);
}

#[test]
fn a_missing_referenced_binding_cannot_be_shared_as_a_value() {
    let mut plans = plans();
    add_pattern(&mut plans, 3);
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    assert_eq!(prepared.classes().len(), 4);
    assert!(
        prepared
            .classes()
            .iter()
            .all(|class| class.members.len() == 1)
    );
    assert!(
        evaluate_plans(&prepared, &bindings, &[true; 4])
            .into_classes()
            .iter()
            .all(Option::is_none)
    );
}

#[test]
fn an_empty_next_file_has_no_stale_native_publication() {
    let plans = plans();
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let mut work = WalkWork::default();
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let mut operator_scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut operator_scratch,
    };
    let bound = bindings
        .iter()
        .cloned()
        .map(Vec::into_boxed_slice)
        .collect::<Vec<_>>();
    let first = evaluate(
        &prepared,
        TreeRows {
            elements: &elements,
            file: FileId(0),
            form: TreeForm::Elements,
        },
        &bound,
        &mut context,
        &[true; 4],
        &mut work,
    );
    assert_eq!(
        first.class(ClassPlace(0)).unwrap().as_ref().unwrap().0,
        [1, 3]
    );
    let empty = ryo_language::domain::elements::types::Elements::default();
    let next = evaluate(
        &prepared,
        TreeRows {
            elements: &empty,
            file: FileId(1),
            form: TreeForm::Elements,
        },
        &bound,
        &mut context,
        &[true; 4],
        &mut work,
    );
    assert!(next.into_classes().iter().all(Option::is_none));
    assert_eq!(
        first.class(ClassPlace(0)).unwrap().as_ref().unwrap().0,
        [1, 3]
    );
}
