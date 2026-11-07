//! Rule policy operates on loaded values without an engine or filesystem.
use super::service;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::linter::{Cell, HostRow, HostText, RelativePath, Rule, RuleId};
use ryo_benchmark_contracts::domain::{
    context::types::{
        ExternalPrefix, PreparedRuleContext, RuleConfiguration, RuleContextTables, TableName,
    },
    reports::{errors::ReportsError, types::PairCount},
    requests::types::{
        CohortSelection, ContextSeal, InputDeclaration, ParserSelection, ProviderSelection,
        RuleContextPreparation, SearchRoots, WorkerRequest, WorkerState,
    },
};
use std::collections::{BTreeMap, BTreeSet};

type Input = fn() -> Result<usize, Failure>;
type Value = usize;
type Failure = Refusal;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    MissingProducer,
    MissingSelection,
    Unexpected,
}
fn id(value: &str) -> RuleId {
    RuleId::try_from(value).unwrap()
}
fn rule() -> Rule {
    Rule {
        query: ryo::query::RyoQlQuery::try_from("let missing = select path as file, span from \"filesystem.read-failure\"; emit missing subject file key(file);").unwrap(),
        observations: vec![], applicability: ryo::linter::RuleApplicability::Always,
        parameters: BTreeMap::new(), include: vec![], exclude: vec![],
    }
}
fn request() -> WorkerRequest {
    WorkerRequest {
        performance: Default::default(),
        selected_rules: CohortSelection::Explicit(vec![id("A001")]),
        rule_observations: InputDeclaration::Undeclared,
        rule_context: RuleContextPreparation::Unconfigured,
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode: Default::default(),
        context_manifest: ContextSeal::Unsealed,
        root: "root".into(),
        rules: "rules".into(),
        query_modules: BTreeMap::new(),
        rule_applicability: BTreeMap::new(),
        parser_configuration: ParserSelection::Unspecified,
        source_language: ProviderSelection::Unspecified,
        files: vec![RelativePath("input.py".into())],
        state: WorkerState::Fresh,
    }
}
fn observes(producer: RuleId) -> ryo::linter::ObservationBinding {
    ryo::linter::ObservationBinding {
        name: ryo::linter::ObservationBindingName("published".into()),
        columns: vec![ryo::linter::ObservationColumn {
            name: ryo::linter::ObservationColumnName("file".into()),
            kind: ryo::linter::ObservationKind::File,
        }],
        producers: vec![ryo::linter::ObservationProducer::Output {
            rule: producer,
            output: ryo::linter::ObservationName("matched".into()),
        }],
    }
}
fn retain_only_the_selected_frontend_and_required_producers() -> Result<Value, Failure> {
    let mut selected = rule();
    selected.observations.push(observes(id("B001")));
    let mut rules = BTreeMap::from([
        (id("A001"), selected),
        (id("B001"), rule()),
        (id("C001"), rule()),
        (id("P001"), rule()),
    ]);
    let cohort = service::retain_producers(&[id("A001")], Some(&id("P001")), &mut rules).unwrap();
    assert_eq!(cohort, BTreeSet::from([id("A001")]));
    assert_eq!(
        rules.keys().cloned().collect::<Vec<_>>(),
        [id("A001"), id("B001"), id("P001")]
    );
    assert_eq!(
        rules[&id("B001")]
            .exclude
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<&str>>(),
        ["**"]
    );
    assert!(rules[&id("A001")].exclude.is_empty());
    assert!(rules[&id("P001")].exclude.is_empty());
    Ok(1)
}
fn missing_producers_refuse_before_retaining_or_excluding_rules() -> Result<Value, Failure> {
    let mut selected = rule();
    selected.observations.push(observes(id("B001")));
    let mut rules = BTreeMap::from([(id("A001"), selected), (id("C001"), rule())]);
    let failure = service::retain_producers(&[id("A001")], None, &mut rules).unwrap_err();
    assert_eq!(rules.len(), 2);
    assert!(rules.values().all(|rule| rule.exclude.is_empty()));
    match failure {
        ReportsError::Invalid(message)
            if message == "selected or required producer has no registered query: B001" =>
        {
            Err(Failure::MissingProducer)
        }
        _ => Err(Failure::Unexpected),
    }
}
fn configuration_projects_exact_declared_tables_and_disabled_pairs() -> Result<Value, Failure> {
    let mut request = request();
    request.files.push(RelativePath("other.py".into()));
    request.rule_context = RuleContextPreparation::Prepared(PreparedRuleContext {
        tables: RuleContextTables {
            selected: TableName("selected".into()),
            disabled: TableName("disabled".into()),
            external: TableName("external".into()),
        },
        configuration: RuleConfiguration {
            external_prefixes: vec![ExternalPrefix("EXT".into())],
            disabled_rules: BTreeMap::from([(request.files[0].clone(), vec![id("A001")])]),
        },
    });
    let mut rules = BTreeMap::from([(id("A001"), rule())]);
    let configured = service::project(&request, &mut rules, PairCount(2)).unwrap();
    assert_eq!(configured.selected_pairs, PairCount(1));
    assert_eq!(configured.hosted.len(), 3);
    assert_eq!(
        configured
            .hosted
            .iter()
            .map(|table| table.name.0.as_ref())
            .collect::<Vec<_>>(),
        ["selected", "disabled", "external"]
    );
    assert_eq!(
        configured.hosted[0].rows,
        [HostRow(vec![Cell::Text(HostText("A001".into()))])]
    );
    assert_eq!(
        configured.hosted[1].rows,
        [HostRow(vec![
            Cell::File(request.files[0].clone()),
            Cell::Text(HostText("A001".into()))
        ])]
    );
    assert_eq!(
        configured.hosted[2].rows,
        [HostRow(vec![Cell::Text(HostText("EXT".into()))])]
    );
    assert_eq!(
        rules[&id("A001")]
            .exclude
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<&str>>(),
        ["input.py"]
    );
    Ok(1)
}

fn missing_context_selection_is_refused() -> Result<Value, Failure> {
    let mut request = request();
    request.rule_context = RuleContextPreparation::Prepared(PreparedRuleContext {
        tables: RuleContextTables {
            selected: TableName("selected".into()),
            disabled: TableName("disabled".into()),
            external: TableName("external".into()),
        },
        configuration: RuleConfiguration {
            external_prefixes: vec![],
            disabled_rules: BTreeMap::new(),
        },
    });
    request.selected_rules = CohortSelection::Frozen;
    let mut rules = BTreeMap::from([(id("A001"), rule())]);
    match service::project(&request, &mut rules, PairCount(2)) {
        Err(ReportsError::Invalid(message))
            if message == "rule context requires the explicit configured selection" =>
        {
            Err(Failure::MissingSelection)
        }
        _ => Err(Failure::Unexpected),
    }
}
fn literal_frontend_paths_do_not_broaden_to_glob_matches() -> Result<Value, Failure> {
    #[cfg(unix)]
    {
        let missing = r"pkg/missing[1]*?{x}é\name.py";
        let readable = r"pkg/missing1anythingQxé\name.py";
        let exclusions =
            service::frontend_exclusions(&[RelativePath(missing.into())].into()).unwrap();
        assert_eq!(
            exclusions.len(),
            1,
            "neither readable nor unrequested paths are exempted"
        );
        let matcher = globset::GlobBuilder::new(exclusions[0].as_ref())
            .literal_separator(true)
            .build()
            .unwrap()
            .compile_matcher();
        assert!(matcher.is_match(missing));
        assert!(!matcher.is_match(readable));
        assert!(!matcher.is_match("pkg/missing1anythingQxéname.py"));
    }
    Ok(1)
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("context requires an explicit selected cohort"),
        word: Word::Refuses,
        input: || missing_context_selection_is_refused,
        expect: Expect::Error(Failure::MissingSelection),
    },
    Case {
        name: CaseName("only selected diagnostics and explicit producer dependencies survive"),
        word: Word::Succeeds,
        input: || retain_only_the_selected_frontend_and_required_producers,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("required producer refusal precedes rule mutation"),
        word: Word::Refuses,
        input: || missing_producers_refuse_before_retaining_or_excluding_rules,
        expect: Expect::Error(Failure::MissingProducer),
    },
    Case {
        name: CaseName("hosted configuration and per-file accounting retain exact data"),
        word: Word::Succeeds,
        input: || configuration_projects_exact_declared_tables_and_disabled_pairs,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("frontend exclusions preserve literal missing path scope"),
        word: Word::Succeeds,
        input: || literal_frontend_paths_do_not_broaden_to_glob_matches,
        expect: Expect::Value(1),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
