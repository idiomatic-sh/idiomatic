//! Raw artifact data cannot issue an audit for a different or incomplete inventory partition.
use crate::domain::suite::service::audited;
use ryo_benchmark_contracts::domain::reports::types::AuthoredInventory;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
fn tool() -> Tool {
    Tool {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::workloads::types::ObservationSelectionPolicy::Declared,
        supporting_rules: vec![],
        rule_observations: Default::default(),
        rule_context: None,
        file_observation: None,
        project_context_mode: Default::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec!["pin".into()],
        deferred: Default::default(),
        pending: Default::default(),
        query_modules: Default::default(),
        rule_applicability: Default::default(),
        parser_configuration: None,
        source_language: None,
        project_search_roots: None,
        defaults: vec!["A001".try_into().unwrap(), "B001".try_into().unwrap()],
    }
}
#[test]
fn exact_disjoint_inventory_is_bound_before_an_audit_is_issued() {
    let a = ryo::linter::RuleId::try_from("A001").unwrap();
    let b = ryo::linter::RuleId::try_from("B001").unwrap();
    let issued = audited(
        &tool(),
        AuthoredInventory {
            implemented: vec![a.clone()],
            missing: vec![b.clone()],
        },
    )
    .unwrap();
    assert_eq!(issued.authored(), &[a]);
    assert_eq!(issued.missing, vec![b]);
}
#[test]
fn duplicate_overlap_undeclared_and_unreported_defaults_refuse_raw_observations() {
    let a = ryo::linter::RuleId::try_from("A001").unwrap();
    let b = ryo::linter::RuleId::try_from("B001").unwrap();
    let foreign = ryo::linter::RuleId::try_from("X001").unwrap();
    for (implemented, missing) in [
        (vec![a.clone(), a.clone()], vec![b.clone()]),
        (vec![a.clone()], vec![a.clone(), b.clone()]),
        (vec![a.clone()], vec![]),
        (vec![a, b], vec![foreign]),
    ] {
        assert!(
            audited(
                &tool(),
                AuthoredInventory {
                    implemented,
                    missing
                }
            )
            .is_err()
        );
    }
}
