//! Exact prepared equivalence and independent native result identity.

use super::value::PreparedWalk;
use crate::domain::{
    coverage::types::RuleIndex,
    positions::types::ClassPlace,
    shared_walk::{
        service_test::{add_pattern, evaluate_plans, plans},
        types::RuleActivity,
    },
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
    let batch = evaluate_plans(&prepared, &bindings, &[RuleActivity::Active; 4]);
    assert_eq!(
        batch.class(ClassPlace(0)).unwrap().as_ref().unwrap().0,
        [1, 3]
    );
    assert_eq!(batch.visits(ClassPlace(0)).0, 2);
    assert_eq!(batch.visits(ClassPlace(1)).0, 0);
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
        evaluate_plans(&prepared, &bindings, &[RuleActivity::Active; 4])
            .class(ClassPlace(0))
            .unwrap()
            .as_ref()
            .unwrap()
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
    assert!(
        evaluate_plans(&prepared, &bindings, &[RuleActivity::Active; 4])
            .into_classes()
            .iter()
            .all(Option::is_none)
    );
}

use ryo_values::domain::query_bindings::types::ParameterBinding;
