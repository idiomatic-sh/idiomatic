//! Query registry and exact skip-accounting contracts.
use super::{EngineLocalAdapter, RuleApplicability, RuleId, linter};
use std::{collections::BTreeMap, path::PathBuf};

fn profile() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().expect(crate::outbound::engine::adapters::local::constants::PROFILE_ROOT);
    let rules = root.path().join(crate::outbound::engine::adapters::local::constants::RULES);
    std::fs::create_dir_all(rules.join(crate::outbound::engine::adapters::local::constants::F401)).expect(crate::outbound::engine::adapters::local::constants::RULE_DIRECTORY);
    std::fs::write(
        rules.join(crate::outbound::engine::adapters::local::constants::F401_QUERY_RYOQL),
        crate::outbound::engine::adapters::local::constants::IMPORT_SHARED_FROM_PYTHON_RUFF_SUPPRESS,
    )
    .expect(crate::outbound::engine::adapters::local::constants::QUERY_SOURCE);
    (root, rules)
}

#[test]
fn worker_reads_explicit_module_text_from_the_frozen_profile() {
    let (root, rules) = profile();
    let module = root.path().join(crate::outbound::engine::adapters::local::constants::FRAGMENTS_SUPPRESS_RYOQL);
    std::fs::create_dir_all(module.parent().expect(crate::outbound::engine::adapters::local::constants::MODULE_PARENT)).expect(crate::outbound::engine::adapters::local::constants::MODULE_FOLDER);
    let text = crate::outbound::engine::adapters::local::constants::INPUT_SOURCE_RELATION_ID_NODE;
    std::fs::write(&module, text).expect(crate::outbound::engine::adapters::local::constants::MODULE_SOURCE);
    let registry = BTreeMap::from([(
        crate::outbound::engine::adapters::local::constants::PYTHON_RUFF_SUPPRESS.to_owned(),
        PathBuf::from(crate::outbound::engine::adapters::local::constants::FRAGMENTS_SUPPRESS_RYOQL),
    )]);
    let queries =
        EngineLocalAdapter::load_rules(&rules, &registry, &BTreeMap::default(), None, None)
            .expect(crate::outbound::engine::adapters::local::constants::MODULE_BUNDLE_IS_LOADED_IN_THE);
    assert_eq!(queries.len(), 1);
    assert_eq!(
        queries.values().next().expect(crate::outbound::engine::adapters::local::constants::RULE).query.root_text(),
        "import shared from \"python/ruff/suppress\";"
    );
    assert_eq!(
        queries.values().next().expect(crate::outbound::engine::adapters::local::constants::RULE).applicability,
        RuleApplicability::Always
    );
}

#[test]
fn selected_rules_receive_the_pinned_syntax_applicability() {
    let (_root, rules) = profile();
    let applicability = BTreeMap::from([(crate::outbound::engine::adapters::local::constants::F401.to_owned(), crate::outbound::engine::adapters::local::constants::ACCEPTED_SYNTAX.to_owned())]);
    let queries =
        EngineLocalAdapter::load_rules(&rules, &BTreeMap::default(), &applicability, None, None)
            .expect(crate::outbound::engine::adapters::local::constants::COMPLETE_APPLICABILITY_MAP_LOADS);
    assert_eq!(
        queries.values().next().expect(crate::outbound::engine::adapters::local::constants::RULE).applicability,
        RuleApplicability::AcceptedSyntax
    );
}

#[test]
fn explicit_applicability_must_match_every_loaded_rule() {
    let (_root, rules) = profile();
    let applicability = BTreeMap::from([
        (crate::outbound::engine::adapters::local::constants::F401.to_owned(), crate::outbound::engine::adapters::local::constants::ACCEPTED_SYNTAX.to_owned()),
        (crate::outbound::engine::adapters::local::constants::F402.to_owned(), crate::outbound::engine::adapters::local::constants::ALWAYS.to_owned()),
    ]);
    let error =
        EngineLocalAdapter::load_rules(&rules, &BTreeMap::default(), &applicability, None, None)
            .expect_err(crate::outbound::engine::adapters::local::constants::EXTRA_APPLICABILITY_ENTRY_IS_REFUSED);
    assert!(error.to_string().contains(crate::outbound::engine::adapters::local::constants::DOES_NOT_EXACTLY_MATCH));
}

#[test]
fn explicit_applicability_rejects_unknown_policy_values() {
    let (_root, rules) = profile();
    let applicability = BTreeMap::from([(crate::outbound::engine::adapters::local::constants::F401.to_owned(), crate::outbound::engine::adapters::local::constants::PARSE_ONLY.to_owned())]);
    let error =
        EngineLocalAdapter::load_rules(&rules, &BTreeMap::default(), &applicability, None, None)
            .expect_err(crate::outbound::engine::adapters::local::constants::UNKNOWN_APPLICABILITY_IS_REFUSED);
    assert!(error.to_string().contains(crate::outbound::engine::adapters::local::constants::UNSUPPORTED_APPLICABILITY));
}

#[test]
fn typed_syntax_skips_require_matching_policy_input_and_projection() -> Result<(), String> {
    let (_root, _directory) = profile();
    let path = linter::RelativePath(crate::outbound::engine::adapters::local::constants::INPUT_PY.into());
    let skipped = linter::Skipped {
        rule: RuleId::try_from(crate::outbound::engine::adapters::local::constants::F401).map_err(|error| error.to_string())?,
        path: path.clone(),
        reason: linter::SkipReason::SyntaxRejected,
    };
    let files = std::collections::BTreeSet::from([path]);
    let policy = BTreeMap::from([(crate::outbound::engine::adapters::local::constants::F401.to_owned(), crate::outbound::engine::adapters::local::constants::ACCEPTED_SYNTAX.to_owned())]);
    let selected = std::collections::BTreeSet::from([
        RuleId::try_from(crate::outbound::engine::adapters::local::constants::F401).map_err(|error| error.to_string())?
    ]);
    EngineLocalAdapter::validate_skips(
        std::slice::from_ref(&skipped),
        &selected,
        &policy,
        &files,
        true,
    )
    .expect(crate::outbound::engine::adapters::local::constants::EXPECTED_SUCCESS);
    let _verified_2 = EngineLocalAdapter::validate_skips(
        std::slice::from_ref(&skipped),
        &selected,
        &policy,
        &files,
        false,
    )
    .expect_err(crate::outbound::engine::adapters::local::constants::EXPECTED_REFUSAL);
    let _verified_3 = EngineLocalAdapter::validate_skips(
        &[skipped.clone(), skipped.clone()],
        &selected,
        &policy,
        &files,
        true,
    )
    .expect_err(crate::outbound::engine::adapters::local::constants::EXPECTED_REFUSAL);

    let _verified_4 = EngineLocalAdapter::validate_skips(
        std::slice::from_ref(&skipped),
        &selected,
        &BTreeMap::new(),
        &files,
        true,
    )
    .expect_err(crate::outbound::engine::adapters::local::constants::EXPECTED_REFUSAL);
    let other_file = std::collections::BTreeSet::from([linter::RelativePath(crate::outbound::engine::adapters::local::constants::ELSE_PY.into())]);
    let _verified_5 =
        EngineLocalAdapter::validate_skips(&[skipped], &selected, &policy, &other_file, true)
            .expect_err(crate::outbound::engine::adapters::local::constants::EXPECTED_REFUSAL);
    Ok(())
}

#[test]
fn worker_refuses_module_paths_outside_the_profile() {
    let (_root, rules) = profile();
    let registry = BTreeMap::from([(
        crate::outbound::engine::adapters::local::constants::PYTHON_RUFF_SUPPRESS.to_owned(),
        PathBuf::from(crate::outbound::engine::adapters::local::constants::OUTSIDE_RYOQL),
    )]);
    let error = EngineLocalAdapter::load_rules(&rules, &registry, &BTreeMap::default(), None, None)
        .expect_err(crate::outbound::engine::adapters::local::constants::TRAVERSAL_IS_REFUSED);
    assert!(error.to_string().contains(crate::outbound::engine::adapters::local::constants::UNSAFE_CORPUS_PATH));
}

#[cfg(unix)]
#[test]
fn worker_refuses_symlinked_module_paths() {
    let (root, rules) = profile();
    std::fs::create_dir_all(root.path().join(crate::outbound::engine::adapters::local::constants::FRAGMENTS)).expect(crate::outbound::engine::adapters::local::constants::MODULE_FOLDER);
    let target = root.path().join(crate::outbound::engine::adapters::local::constants::OUTSIDE_RYOQL_2);
    std::fs::write(&target, crate::outbound::engine::adapters::local::constants::MODULE_BYTES).expect(crate::outbound::engine::adapters::local::constants::MODULE_SOURCE);
    std::os::unix::fs::symlink(&target, root.path().join(crate::outbound::engine::adapters::local::constants::FRAGMENTS_SUPPRESS_RYOQL))
        .expect(crate::outbound::engine::adapters::local::constants::SYMLINK_FIXTURE);
    let registry = BTreeMap::from([(
        crate::outbound::engine::adapters::local::constants::PYTHON_RUFF_SUPPRESS.to_owned(),
        PathBuf::from(crate::outbound::engine::adapters::local::constants::FRAGMENTS_SUPPRESS_RYOQL),
    )]);
    let error = EngineLocalAdapter::load_rules(&rules, &registry, &BTreeMap::default(), None, None)
        .expect_err(crate::outbound::engine::adapters::local::constants::SYMLINK_IS_REFUSED);
    assert!(error.to_string().contains(crate::outbound::engine::adapters::local::constants::QUERY_MODULE_IS_A_SYMLINK));
}

#[test]
fn named_input_declarations_attach_to_registered_definitions_and_reject_unknown_consumers() {
    use ryo::linter::{
        ObservationBinding, ObservationBindingName, ObservationColumn, ObservationColumnName,
        ObservationKind, ObservationName, ObservationProducer,
    };
    let (_root, rules) = profile();
    let consumer = RuleId::try_from(crate::outbound::engine::adapters::local::constants::F401).unwrap();
    let binding = ObservationBinding {
        name: ObservationBindingName(crate::outbound::engine::adapters::local::constants::PUBLISHED.into()),
        columns: vec![ObservationColumn {
            name: ObservationColumnName(crate::outbound::engine::adapters::local::constants::FILE.into()),
            kind: ObservationKind::File,
        }],
        producers: vec![ObservationProducer::Output {
            rule: RuleId::try_from(crate::outbound::engine::adapters::local::constants::B002).unwrap(),
            output: ObservationName(crate::outbound::engine::adapters::local::constants::MATCHED.into()),
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
    declarations.insert(RuleId::try_from(crate::outbound::engine::adapters::local::constants::TYPO).unwrap(), vec![binding]);
    let error = EngineLocalAdapter::load_rules(
        &rules,
        &BTreeMap::new(),
        &BTreeMap::new(),
        None,
        Some(&declarations),
    )
    .unwrap_err();
    assert!(error.to_string().contains(crate::outbound::engine::adapters::local::constants::UNREGISTERED_CONSUMER));
}
