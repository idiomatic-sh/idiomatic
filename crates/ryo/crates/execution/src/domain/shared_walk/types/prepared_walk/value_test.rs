//! Exact prepared equivalence and independent native result identity.

use super::value::PreparedWalk;
use crate::domain::{
    coverage::types::RuleIndex,
    positions::types::ClassPlace,
    shared_walk::service_test::{add_pattern, evaluate_plans, plans},
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
        ColumnIndex, Comparison, Input, Operand, ParameterId, QueryLiteral, Test,
    };
    use ryo_values::domain::plans::types::Operation;
    let mut authored = plans();
    let first = Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Parameter(ParameterId(0)),
        origin: None,
    };
    let second = Comparison {
        test: Test::NotEqual,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("other".into()))),
        origin: None,
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
