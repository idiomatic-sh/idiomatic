//! Exact prepared equivalence and independent native result identity.

use super::value::PreparedWalk;
use crate::domain::{
    coverage::types::RuleIndex,
    shared_walk::service_test::{add_pattern, evaluate_plans, plans},
    step_classes::types::ClassPlace,
};
use ryo_values::domain::plan_parts::types::LiteralText;

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
    let batch = evaluate_plans(
        &prepared,
        &bindings,
        &[ClassPlace(0), ClassPlace(1), ClassPlace(2)],
    );
    assert_eq!(batch.class(ClassPlace(0)).unwrap().positions.0, [1, 3]);
    assert_eq!(batch.visits(ClassPlace(0)).0, 2);
    assert_eq!(batch.visits(ClassPlace(1)).0, 1);
    assert_eq!(batch.class(ClassPlace(1)).unwrap().positions.0, [2]);
    // Nodes have no token text; the ordinary Select contract reads that as empty.
    // The inbound kinds control independently checks root-only empty text selection.
    assert_eq!(batch.class(ClassPlace(2)).unwrap().positions.0, [0]);
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
        evaluate_plans(
            &prepared,
            &bindings,
            &[ClassPlace(0), ClassPlace(1), ClassPlace(2)]
        )
        .class(ClassPlace(0))
        .unwrap()
        .positions
        .0,
        [1, 3]
    );
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
    let batch = evaluate_plans(
        &prepared,
        &bindings,
        &[ClassPlace(0), ClassPlace(1), ClassPlace(2), ClassPlace(3)],
    );
    assert!(matches!(
        batch.class(ClassPlace(0)).map(|result| &result.completion),
        Some(Err(_))
    ));
    assert_eq!(batch.class(ClassPlace(1)).unwrap().positions.0, [2]);
    assert!(matches!(
        batch.class(ClassPlace(2)).map(|result| &result.completion),
        Some(Err(_))
    ));
}

use ryo_values::domain::query_bindings::types::ParameterBinding;

#[test]
fn predicate_prefixes_share_only_the_referenced_bound_values_in_authored_order() {
    use ryo_values::domain::plan_parts::types::{
        ColumnIndex, Input, Operand, ParameterId, PlanComparison, QueryLiteral, Test,
    };
    use ryo_values::domain::plans::types::Operation;
    let mut authored = plans();
    let first = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Parameter(ParameterId(0)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let second = PlanComparison {
        test: Test::NotEqual,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("other".into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    for (at, plan) in authored.iter_mut().enumerate() {
        plan.steps[0].operation = Operation::Select {
            from: Input::Relation(ryo_relations::domain::relations::types::Relation::SyntaxTree),
            comparisons: if at == 3 {
                vec![second.clone(), first.clone()]
            } else {
                vec![first.clone(), second.clone()]
            },
            kept: vec![],
        };
    }
    let bindings = vec![
        vec![ParameterBinding::Text(LiteralText("a".into()))],
        vec![ParameterBinding::Text(LiteralText("b".into()))],
        vec![
            ParameterBinding::Text(LiteralText("a".into())),
            ParameterBinding::Numbers(vec![]),
        ],
        vec![ParameterBinding::Text(LiteralText("a".into()))],
    ];
    let prepared = PreparedWalk::compile(&authored, &bindings);
    let class = |rule| prepared.class_of(RuleIndex(rule)).unwrap()[0].unwrap();
    assert_eq!(class(0), class(2));
    assert_ne!(
        prepared.prefixes(class(0))[0],
        prepared.prefixes(class(1))[0]
    );
    assert_ne!(
        prepared.prefixes(class(0))[1],
        prepared.prefixes(class(3))[0]
    );
    let missing = PreparedWalk::compile(&authored, &vec![vec![]; 4]);
    assert_ne!(
        missing.prefixes(ClassPlace(0))[0],
        missing.prefixes(ClassPlace(1))[0]
    );
}

/// The shared walk of `plans` bound to the one executable program compiled from them.
fn bound(plans: &[ryo_values::domain::plans::types::Plan]) -> PreparedWalk {
    use crate::domain::source_partitions::types::source_partitions::value::SourcePartitions;
    use ryo_values::domain::{
        query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
        rule_requests::types::{RuleParameters, RuleSpec},
        runs::types::{RuleApplicability, RuleId},
    };
    let rules = (0..plans.len())
        .map(|rule| RuleSpec {
            id: RuleId(format!("rule-{rule}").into()),
            query: QueryBundle {
                root: QueryDocumentSource {
                    id: QuerySourceId(format!("rule-{rule}").into()),
                    text: "".into(),
                },
                modules: Default::default(),
            },
            applicability: RuleApplicability::Always,
            given: vec![],
            observations: vec![],
            spelled: RuleParameters("".into()),
            include: vec![],
            exclude: vec![],
        })
        .collect::<Vec<_>>();
    let bindings = vec![vec![]; plans.len()];
    let operators = crate::domain::file_program::service::operators::service::prepare(
        plans,
        &bindings,
        &[],
        &ryo_relations::domain::relations::service::kinds,
        &mut |_| Ok(()),
    )
    .expect("the plans prepare");
    let borrowed = plans.iter().map(Ok).collect::<Vec<_>>();
    let partitions = SourcePartitions::new(
        &rules,
        &borrowed,
        &bindings,
        &[],
        &vec![vec![]; plans.len()],
    )
    .expect("the plans partition");
    let graph = crate::domain::file_program::service::compile_prepared(plans, &operators);
    let program = crate::domain::executable_program::service::compile(
        plans,
        graph,
        &partitions,
        &operators,
        ryo_operators::domain::patterns::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
    )
    .expect("the plans compile");
    let mut walk = PreparedWalk::compile(plans, &bindings);
    walk.retain_program_consumers(&program)
        .expect("the program routes each step once");
    walk
}

#[test]
fn a_selection_over_a_step_is_not_a_walk_head() {
    use crate::domain::shared_walk::service_test::plan;
    use ryo_values::domain::plan_parts::types::{Input, Operand, QueryLiteral, StepId, StepScope};
    use ryo_values::domain::plans::types::{Operation, PlanStep};
    let mut over_a_step = plan("word");
    let Operation::Select { comparisons, .. } = &mut over_a_step.steps[0].operation else {
        panic!("the fixture builds a Select");
    };
    let word = comparisons.remove(0);
    let mut other = word.clone();
    other.right = Operand::Literal(QueryLiteral::Text(LiteralText("other".into())));
    for comparison in [word, other] {
        over_a_step.steps.push(PlanStep {
            operation: Operation::Select {
                from: Input::Step(StepId(0)),
                comparisons: vec![comparison],
                kept: Vec::new(),
            },
            scope: StepScope::PerFile,
        });
    }
    over_a_step.steps.push(PlanStep {
        operation: Operation::Union {
            left: Input::Step(StepId(1)),
            right: Input::Step(StepId(2)),
        },
        scope: StepScope::PerFile,
    });
    over_a_step.emit.step = StepId(3);
    let census = bound(&[over_a_step]).census();
    assert_eq!(
        census.shared.0, 1,
        "only the selection over the tree heads a walk"
    );
    assert_eq!(
        census.dependent.0, 2,
        "each selection over the step reads that step's rows"
    );
}

#[test]
fn a_row_dropping_projection_is_not_a_walk_head() {
    use crate::domain::shared_walk::service_test::plan;
    use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, RepeatedRows};
    use ryo_values::domain::plans::types::Operation;
    let mut dropping = plan("word");
    dropping.steps[0].operation = Operation::Project {
        from: Input::Relation(ryo_relations::domain::relations::types::Relation::SyntaxTree),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    };
    let census = bound(&[dropping]).census();
    assert_eq!(
        census.shared.0, 0,
        "a row-dropping projection shares no walk"
    );
    assert!(census.classes.is_empty());
}
