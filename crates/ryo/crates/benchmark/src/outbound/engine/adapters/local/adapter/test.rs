//! Query registry and exact skip-accounting contracts.
use super::{EngineLocalAdapter, RuleApplicability, RuleId, linter};
use crate::outbound::engine::adapters::local::constants;
use std::{collections::BTreeMap, path::PathBuf};

fn profile() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().expect("profile root");
    let rules = root.path().join("rules");
    std::fs::create_dir_all(rules.join("F401")).expect("rule directory");
    std::fs::write(
        rules.join("F401/query.ryoql"),
        "import shared from \"python/ruff/suppress\";",
    )
    .expect("query source");
    (root, rules)
}

#[test]
fn worker_reads_explicit_module_text_from_the_frozen_profile() {
    let (root, rules) = profile();
    let module = root.path().join("fragments/suppress.ryoql");
    std::fs::create_dir_all(module.parent().expect("module parent")).expect("module folder");
    let text = "input source: relation<id: node>;\n";
    std::fs::write(&module, text).expect(constants::MODULE_SOURCE);
    let registry = BTreeMap::from([(
        "python/ruff/suppress".to_owned(),
        PathBuf::from("fragments/suppress.ryoql"),
    )]);
    let queries =
        EngineLocalAdapter::load_rules(&rules, &registry, &BTreeMap::default(), None, None)
            .expect("module bundle is loaded in the worker");
    assert_eq!(queries.len(), 1);
    assert_eq!(
        queries.values().next().expect("rule").query.root_text(),
        "import shared from \"python/ruff/suppress\";"
    );
    assert_eq!(
        queries.values().next().expect("rule").applicability,
        RuleApplicability::Always
    );
}

#[test]
fn selected_rules_receive_the_pinned_syntax_applicability() {
    let (_root, rules) = profile();
    let applicability =
        BTreeMap::from([("F401".to_owned(), constants::ACCEPTED_SYNTAX.to_owned())]);
    let queries =
        EngineLocalAdapter::load_rules(&rules, &BTreeMap::default(), &applicability, None, None)
            .expect("complete applicability map loads");
    assert_eq!(
        queries.values().next().expect("rule").applicability,
        RuleApplicability::AcceptedSyntax
    );
}

#[test]
fn explicit_applicability_must_match_every_loaded_rule() {
    let (_root, rules) = profile();
    let applicability = BTreeMap::from([
        ("F401".to_owned(), constants::ACCEPTED_SYNTAX.to_owned()),
        ("F402".to_owned(), constants::ALWAYS.to_owned()),
    ]);
    let error =
        EngineLocalAdapter::load_rules(&rules, &BTreeMap::default(), &applicability, None, None)
            .expect_err("extra applicability entry is refused");
    assert!(error.to_string().contains("does not exactly match"));
}

#[test]
fn explicit_applicability_rejects_unknown_policy_values() {
    let (_root, rules) = profile();
    let applicability = BTreeMap::from([("F401".to_owned(), "parse-only".to_owned())]);
    let error =
        EngineLocalAdapter::load_rules(&rules, &BTreeMap::default(), &applicability, None, None)
            .expect_err("unknown applicability is refused");
    assert!(error.to_string().contains("unsupported applicability"));
}

#[test]
fn typed_syntax_skips_require_matching_policy_input_and_projection() -> Result<(), String> {
    let (_root, _directory) = profile();
    let path = linter::RelativePath("input.py".into());
    let skipped = linter::Skipped {
        rule: RuleId::try_from("F401").map_err(|error| error.to_string())?,
        path: path.clone(),
        reason: linter::SkipReason::SyntaxRejected,
    };
    let files = std::collections::BTreeSet::from([path]);
    let policy = BTreeMap::from([("F401".to_owned(), constants::ACCEPTED_SYNTAX.to_owned())]);
    let selected = std::collections::BTreeSet::from([
        RuleId::try_from("F401").map_err(|error| error.to_string())?
    ]);
    EngineLocalAdapter::validate_skips(
        std::slice::from_ref(&skipped),
        &selected,
        &policy,
        &files,
        true,
    )
    .expect("expected success");
    let _verified_2 = EngineLocalAdapter::validate_skips(
        std::slice::from_ref(&skipped),
        &selected,
        &policy,
        &files,
        false,
    )
    .expect_err("expected refusal");
    let _verified_3 = EngineLocalAdapter::validate_skips(
        &[skipped.clone(), skipped.clone()],
        &selected,
        &policy,
        &files,
        true,
    )
    .expect_err("expected refusal");

    let _verified_4 = EngineLocalAdapter::validate_skips(
        std::slice::from_ref(&skipped),
        &selected,
        &BTreeMap::new(),
        &files,
        true,
    )
    .expect_err("expected refusal");
    let other_file = std::collections::BTreeSet::from([linter::RelativePath("else.py".into())]);
    let _verified_5 =
        EngineLocalAdapter::validate_skips(&[skipped], &selected, &policy, &other_file, true)
            .expect_err("expected refusal");
    Ok(())
}

#[test]
fn worker_refuses_module_paths_outside_the_profile() {
    let (_root, rules) = profile();
    let registry = BTreeMap::from([(
        "python/ruff/suppress".to_owned(),
        PathBuf::from("../outside.ryoql"),
    )]);
    let error = EngineLocalAdapter::load_rules(&rules, &registry, &BTreeMap::default(), None, None)
        .expect_err("traversal is refused");
    assert!(error.to_string().contains("unsafe corpus path"));
}

#[cfg(unix)]
#[test]
fn worker_refuses_symlinked_module_paths() {
    let (root, rules) = profile();
    std::fs::create_dir_all(root.path().join("fragments")).expect("module folder");
    let target = root.path().join("outside.ryoql");
    std::fs::write(&target, "module bytes").expect(constants::MODULE_SOURCE);
    std::os::unix::fs::symlink(&target, root.path().join("fragments/suppress.ryoql"))
        .expect("symlink fixture");
    let registry = BTreeMap::from([(
        "python/ruff/suppress".to_owned(),
        PathBuf::from("fragments/suppress.ryoql"),
    )]);
    let error = EngineLocalAdapter::load_rules(&rules, &registry, &BTreeMap::default(), None, None)
        .expect_err("symlink is refused");
    assert!(error.to_string().contains("query module is a symlink"));
}

#[test]
fn named_input_declarations_attach_to_registered_definitions_and_reject_unknown_consumers() {
    use ryo::linter::{
        ObservationBinding, ObservationBindingName, ObservationColumn, ObservationColumnName,
        ObservationKind, ObservationName, ObservationProducer,
    };
    let (_root, rules) = profile();
    let consumer = RuleId::try_from("F401").unwrap();
    let binding = ObservationBinding {
        name: ObservationBindingName("published".into()),
        columns: vec![ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: ObservationKind::File,
        }],
        producers: vec![ObservationProducer::Output {
            rule: RuleId::try_from("B002").unwrap(),
            output: ObservationName("matched".into()),
        }],
    };
    let mut declarations = ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations::from(BTreeMap::from([(consumer.clone(), vec![binding.clone()])]));
    let queries = EngineLocalAdapter::load_rules(
        &rules,
        &BTreeMap::new(),
        &BTreeMap::new(),
        None,
        Some(&declarations),
    )
    .unwrap();
    assert_eq!(queries[&consumer].observations, vec![binding.clone()]);
    declarations.insert(RuleId::try_from("typo").unwrap(), vec![binding]);
    let error = EngineLocalAdapter::load_rules(
        &rules,
        &BTreeMap::new(),
        &BTreeMap::new(),
        None,
        Some(&declarations),
    )
    .unwrap_err();
    assert!(error.to_string().contains("unregistered consumer"));
}
