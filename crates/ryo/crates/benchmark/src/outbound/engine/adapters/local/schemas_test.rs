//! Query registry and exact skip-accounting contracts.
use super::{load_rules, refuse_incomplete, source_inputs, validate_skips};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::engine::adapters::local::constants;
use ryo::linter::{self, RuleApplicability, RuleId};
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, QueryModuleName, RuleLabel,
};
use std::{collections::BTreeMap, path::PathBuf};

/// Create a symbolic link to a file where the platform supports it.
#[cfg(unix)]
fn link(target: &std::path::Path, path: &std::path::Path) {
    std::os::unix::fs::symlink(target, path).expect("symlink fixture");
}
/// Create a symbolic link to a file where the platform supports it.
#[cfg(not(unix))]
fn link(target: &std::path::Path, path: &std::path::Path) {
    std::os::windows::fs::symlink_file(target, path).expect("symlink fixture");
}

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

fn worker_reads_explicit_module_text_from_the_frozen_profile() {
    let (root, rules) = profile();
    let module = root.path().join("fragments/suppress.ryoql");
    std::fs::create_dir_all(module.parent().expect("module parent")).expect("module folder");
    let text = "input source: relation<id: node>;\n";
    std::fs::write(&module, text).expect(constants::MODULE_SOURCE);
    let registry = BTreeMap::from([(
        QueryModuleName("python/ruff/suppress".to_owned()),
        PathBuf::from("fragments/suppress.ryoql"),
    )]);
    let queries = load_rules(&rules, &registry, &BTreeMap::default(), None, None)
        .expect("module bundle is loaded in the worker");
    assert_eq!(queries.len(), 1);
    assert_eq!(
        &**queries.values().next().expect("rule").query.root_text(),
        "import shared from \"python/ruff/suppress\";"
    );
    assert_eq!(
        queries.values().next().expect("rule").applicability,
        RuleApplicability::Always
    );
}

fn selected_rules_receive_the_pinned_syntax_applicability() {
    let (_root, rules) = profile();
    let applicability = BTreeMap::from([(
        RuleLabel("F401".to_owned()),
        ApplicabilityPolicy(constants::ACCEPTED_SYNTAX.to_owned()),
    )]);
    let queries = load_rules(&rules, &BTreeMap::default(), &applicability, None, None)
        .expect("complete applicability map loads");
    assert_eq!(
        queries.values().next().expect("rule").applicability,
        RuleApplicability::AcceptedSyntax
    );
}

fn explicit_applicability_must_match_every_loaded_rule() {
    let (_root, rules) = profile();
    let applicability = BTreeMap::from([
        (
            RuleLabel("F401".to_owned()),
            ApplicabilityPolicy(constants::ACCEPTED_SYNTAX.to_owned()),
        ),
        (
            RuleLabel("F402".to_owned()),
            ApplicabilityPolicy(constants::ALWAYS.to_owned()),
        ),
    ]);
    let error = load_rules(&rules, &BTreeMap::default(), &applicability, None, None)
        .expect_err("extra applicability entry is refused");
    assert!(error.to_string().contains("does not exactly match"));
}

fn explicit_applicability_rejects_unknown_policy_values() {
    let (_root, rules) = profile();
    let applicability = BTreeMap::from([(
        RuleLabel("F401".to_owned()),
        ApplicabilityPolicy("parse-only".to_owned()),
    )]);
    let error = load_rules(&rules, &BTreeMap::default(), &applicability, None, None)
        .expect_err("unknown applicability is refused");
    assert!(error.to_string().contains("unsupported applicability"));
}

fn typed_syntax_skips_require_matching_policy_input_and_projection() {
    let (_root, _directory) = profile();
    let path = linter::RelativePath("input.py".into());
    let skipped = linter::Skipped {
        rule: RuleId::try_from("F401").expect("fixture rule"),
        path: path.clone(),
        reason: linter::SkipReason::SyntaxRejected,
    };
    let files = std::collections::BTreeSet::from([path]);
    let policy = BTreeMap::from([(
        RuleLabel("F401".to_owned()),
        ApplicabilityPolicy(constants::ACCEPTED_SYNTAX.to_owned()),
    )]);
    let selected =
        std::collections::BTreeSet::from([RuleId::try_from("F401").expect("fixture rule")]);
    validate_skips(
        std::slice::from_ref(&skipped),
        &selected,
        &policy,
        &files,
        true,
    )
    .expect("expected success");
    let _verified_2 = validate_skips(
        std::slice::from_ref(&skipped),
        &selected,
        &policy,
        &files,
        false,
    )
    .expect_err("expected refusal");
    let _verified_3 = validate_skips(
        &[skipped.clone(), skipped.clone()],
        &selected,
        &policy,
        &files,
        true,
    )
    .expect_err("expected refusal");

    let _verified_4 = validate_skips(
        std::slice::from_ref(&skipped),
        &selected,
        &BTreeMap::new(),
        &files,
        true,
    )
    .expect_err("expected refusal");
    let other_file = std::collections::BTreeSet::from([linter::RelativePath("else.py".into())]);
    let _verified_5 = validate_skips(
        std::slice::from_ref(&skipped),
        &selected,
        &policy,
        &other_file,
        true,
    )
    .expect_err("expected refusal");
    // A rule the run did not keep, as selected or as a producer of a selected rule, has no skip to explain.
    let _verified_6 = validate_skips(
        &[skipped],
        &std::collections::BTreeSet::new(),
        &policy,
        &files,
        true,
    )
    .expect_err("expected refusal");
}

fn worker_refuses_module_paths_outside_the_profile() {
    let (_root, rules) = profile();
    let registry = BTreeMap::from([(
        QueryModuleName("python/ruff/suppress".to_owned()),
        PathBuf::from("../outside.ryoql"),
    )]);
    let error = load_rules(&rules, &registry, &BTreeMap::default(), None, None)
        .expect_err("traversal is refused");
    assert!(error.to_string().contains("unsafe corpus path"));
}

fn worker_refuses_symlinked_module_paths() {
    let (root, rules) = profile();
    std::fs::create_dir_all(root.path().join("fragments")).expect("module folder");
    let target = root.path().join("outside.ryoql");
    std::fs::write(&target, "module bytes").expect(constants::MODULE_SOURCE);
    link(&target, &root.path().join("fragments/suppress.ryoql"));
    let registry = BTreeMap::from([(
        QueryModuleName("python/ruff/suppress".to_owned()),
        PathBuf::from("fragments/suppress.ryoql"),
    )]);
    let error = load_rules(&rules, &registry, &BTreeMap::default(), None, None)
        .expect_err("symlink is refused");
    assert!(error.to_string().contains("query module is a symlink"));
}

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
    let queries = load_rules(
        &rules,
        &BTreeMap::new(),
        &BTreeMap::new(),
        None,
        Some(&declarations),
    )
    .unwrap();
    assert_eq!(queries[&consumer].observations, vec![binding.clone()]);
    declarations.insert(RuleId::try_from("typo").unwrap(), vec![binding]);
    let error = load_rules(
        &rules,
        &BTreeMap::new(),
        &BTreeMap::new(),
        None,
        Some(&declarations),
    )
    .unwrap_err();
    assert!(error.to_string().contains("unregistered consumer"));
}

/// A rule left undecided in a file refuses the candidate even when every file was read: unknowns are never a checked answer.
fn unknowns_without_unread_refuse_the_candidate() {
    let none = std::collections::BTreeSet::new();
    let refused = refuse_incomplete(
        &["rule R1 reads `identifier` in part in input.txt"],
        &[],
        &none,
        1,
    );
    assert!(
        matches!(&refused, Err(ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(message))
            if message.starts_with("incomplete candidate:") && message.contains("unknown_count=1") && message.contains("unread_count=0")),
        "an unknown with nothing unread is refused: {refused:?}"
    );
    let undecided: &[&str] = &[];
    assert_eq!(
        refuse_incomplete(undecided, &[], &none, 1).map_err(|error| error.to_string()),
        Ok(()),
        "nothing undecided and nothing unread is complete"
    );
}

/// A requested source that is not on disk, and that context validation did not declare absent, fails the read with `NotFound`.
fn an_undeclared_missing_source_fails_its_read() {
    let root = tempfile::tempdir().expect("a root");
    let request = ryo_benchmark_contracts::domain::requests::types::WorkerRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen,
        rule_observations:
            ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context:
            ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots:
            ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode: Default::default(),
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: root.path().into(),
        rules: root.path().into(),
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration:
            ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language:
            ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        files: vec![linter::RelativePath("missing.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    };
    let read = source_inputs(&request, &std::collections::BTreeSet::new());
    assert!(
        matches!(&read, Err(ryo_benchmark_contracts::domain::reports::errors::ReportsError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound),
        "an undeclared missing source is a NotFound read: {:?}",
        read.map(|_| ())
    );
    let declared = std::collections::BTreeSet::from([linter::RelativePath("missing.py".into())]);
    let (sources, accepted) =
        source_inputs(&request, &declared).expect("a declared absence is not read");
    assert!(sources.is_empty(), "a declared absence supplies no bytes");
    assert_eq!(
        accepted, declared,
        "the declared absence keeps its requested path"
    );
}

type Scenario = fn();
type Input = Scenario;
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("worker_reads_explicit_module_text_from_the_frozen_profile"),
        word: Word::Succeeds,
        input: || worker_reads_explicit_module_text_from_the_frozen_profile as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("selected_rules_receive_the_pinned_syntax_applicability"),
        word: Word::Succeeds,
        input: || selected_rules_receive_the_pinned_syntax_applicability as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("explicit_applicability_must_match_every_loaded_rule"),
        word: Word::Succeeds,
        input: || explicit_applicability_must_match_every_loaded_rule as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("explicit_applicability_rejects_unknown_policy_values"),
        word: Word::Succeeds,
        input: || explicit_applicability_rejects_unknown_policy_values as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("typed_syntax_skips_require_matching_policy_input_and_projection"),
        word: Word::Succeeds,
        input: || typed_syntax_skips_require_matching_policy_input_and_projection as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("worker_refuses_module_paths_outside_the_profile"),
        word: Word::Succeeds,
        input: || worker_refuses_module_paths_outside_the_profile as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("worker_refuses_symlinked_module_paths"),
        word: Word::Succeeds,
        input: || worker_refuses_symlinked_module_paths as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "named_input_declarations_attach_to_registered_definitions_and_reject_unknown_consumers",
        ),
        word: Word::Succeeds,
        input: || {
            named_input_declarations_attach_to_registered_definitions_and_reject_unknown_consumers
                as Scenario
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unknowns_without_unread_refuse_the_candidate"),
        word: Word::Succeeds,
        input: || unknowns_without_unread_refuse_the_candidate as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_undeclared_missing_source_fails_its_read"),
        word: Word::Succeeds,
        input: || an_undeclared_missing_source_fails_its_read as Scenario,
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
