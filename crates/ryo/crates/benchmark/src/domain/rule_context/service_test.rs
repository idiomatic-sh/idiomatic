//! Declared rule configuration is resolved once, checked against the request, and counted exactly.
use super::constants;
use super::service::{admit_configuration, check_selection, literal_glob, resolve, selected_pairs};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::linter::{GivenProjectInventory, RelativePath, RuleId};
use ryo_benchmark_contracts::domain::context::types::{
    PreparedRuleContext, RuleConfiguration, RuleContextDeclaration, RuleContextTables, TableName,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::workloads::types::ConfigurationOverride;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration,
    PerformanceOptions, RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
};
use ryo_benchmark_contracts::domain::workloads::types::{
    RuleConfigurationSupport, RuleContextProfile,
};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// What each scenario resolves, checks or counts.
#[derive(Clone, Copy)]
enum Scenario {
    InheritedWithoutProfile,
    ProfileDefaultsApply,
    PairsAfterDisabled,
    LiteralGlobEscapes,
    OverrideWithoutProfile,
    DuplicateTables,
    FrozenCohort,
    DisabledOutsideCohort,
    UnsupportedContestant,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no profile declaration and no override leave the request unconfigured"),
        word: Word::Succeeds,
        input: || Scenario::InheritedWithoutProfile,
        expect: Expect::Value(Cow::Borrowed("unconfigured")),
    },
    Case {
        name: CaseName("a declared profile prepares its own defaults"),
        word: Word::Succeeds,
        input: || Scenario::ProfileDefaultsApply,
        expect: Expect::Value(Cow::Borrowed("prepared")),
    },
    Case {
        name: CaseName("disabled pairs are subtracted from the selected pairs"),
        word: Word::Succeeds,
        input: || Scenario::PairsAfterDisabled,
        // 3 selected rules x 1 file - 2 disabled pairs = 1 pair left.
        expect: Expect::Value(Cow::Borrowed("1")),
    },
    Case {
        name: CaseName("a path is escaped as a literal glob"),
        word: Word::Succeeds,
        input: || Scenario::LiteralGlobEscapes,
        expect: Expect::Value(Cow::Borrowed("pkg/[[]1[]].py")),
    },
    Case {
        name: CaseName("an override without a profile declaration is refused"),
        word: Word::Refuses,
        input: || Scenario::OverrideWithoutProfile,
        expect: Expect::Error(Cow::Borrowed(
            constants::CASE_RULE_CONFIGURATION_REQUIRES_THE_PROFILE,
        )),
    },
    Case {
        name: CaseName("repeated table names are refused"),
        word: Word::Refuses,
        input: || Scenario::DuplicateTables,
        expect: Expect::Error(Cow::Borrowed(
            constants::RULE_CONFIGURATION_TABLES_MUST_BE_DISTINCT,
        )),
    },
    Case {
        name: CaseName("a frozen cohort cannot hold a configuration"),
        word: Word::Refuses,
        input: || Scenario::FrozenCohort,
        expect: Expect::Error(Cow::Borrowed(
            constants::RULE_CONTEXT_REQUIRES_AN_EXPLICIT_SELECTED,
        )),
    },
    Case {
        name: CaseName("a disabled rule outside the cohort is refused"),
        word: Word::Refuses,
        input: || Scenario::DisabledOutsideCohort,
        expect: Expect::Error(Cow::Borrowed(constants::DISABLED_RULES_MUST_BELONG_TO_THE)),
    },
    Case {
        name: CaseName("a contestant without the explicit contract refuses the configuration"),
        word: Word::Refuses,
        input: || Scenario::UnsupportedContestant,
        expect: Expect::Error(Cow::Borrowed(
            constants::CONTESTANT_DOES_NOT_IMPLEMENT_THE_DECLARED,
        )),
    },
];

fn id(text: &str) -> RuleId {
    RuleId::try_from(text).unwrap()
}

fn file(text: &str) -> RelativePath {
    RelativePath(text.into())
}

fn tables(names: [&str; 3]) -> RuleContextTables {
    RuleContextTables {
        selected: TableName(names[0].into()),
        disabled: TableName(names[1].into()),
        external: TableName(names[2].into()),
    }
}

fn declaration(names: [&str; 3]) -> RuleContextProfile {
    RuleContextProfile::Declared(RuleContextDeclaration {
        tables: tables(names),
        defaults: RuleConfiguration {
            external_prefixes: Vec::new(),
            disabled_rules: BTreeMap::new(),
        },
    })
}

fn prepared(disabled: &[(&str, &[&str])]) -> PreparedRuleContext {
    PreparedRuleContext {
        tables: tables(["selected", "disabled", "external"]),
        configuration: RuleConfiguration {
            external_prefixes: Vec::new(),
            disabled_rules: disabled
                .iter()
                .map(|(path, rules)| (file(path), rules.iter().map(|rule| id(rule)).collect()))
                .collect(),
        },
    }
}

fn request() -> RunRequest {
    RunRequest {
        performance: PerformanceOptions::default(),
        selected_rules: CohortSelection::Explicit(vec![id("A")]),
        rule_observations: InputDeclaration::Undeclared,
        rule_context: RuleContextPreparation::Prepared(prepared(&[])),
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: GivenProjectInventory::default(),
        staged_sources: StagedSources::Unlisted,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context_manifest: ContextSeal::Unsealed,
        root: PathBuf::from("root"),
        files: Vec::new(),
        state: PathBuf::from("state"),
    }
}

fn describe(preparation: &RuleContextPreparation) -> Value {
    match preparation {
        RuleContextPreparation::Unconfigured => Cow::Borrowed("unconfigured"),
        RuleContextPreparation::Prepared(_) => Cow::Borrowed("prepared"),
    }
}

fn failure(error: &ReportsError) -> Failure {
    Cow::Owned(error.to_string())
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::InheritedWithoutProfile => resolve(
            &RuleContextProfile::Undeclared,
            &ConfigurationOverride::Inherited,
        )
        .map(|preparation| describe(&preparation))
        .map_err(|error| failure(&error)),
        Scenario::ProfileDefaultsApply => resolve(
            &declaration(["selected", "disabled", "external"]),
            &ConfigurationOverride::Inherited,
        )
        .map(|preparation| describe(&preparation))
        .map_err(|error| failure(&error)),
        Scenario::PairsAfterDisabled => selected_pairs(
            &prepared(&[("a.py", &["A", "B"][..])]),
            &[id("A"), id("B"), id("C")],
            &[file("a.py")],
        )
        .map(|pairs| Cow::Owned(pairs.0.to_string()))
        .map_err(|error| failure(&error)),
        Scenario::LiteralGlobEscapes => Ok(Cow::Owned(literal_glob(&file("pkg/[1].py")).0)),
        Scenario::OverrideWithoutProfile => resolve(
            &RuleContextProfile::Undeclared,
            &ConfigurationOverride::Overridden(RuleConfiguration {
                external_prefixes: Vec::new(),
                disabled_rules: BTreeMap::new(),
            }),
        )
        .map(|preparation| describe(&preparation))
        .map_err(|error| failure(&error)),
        Scenario::DuplicateTables => resolve(
            &declaration(["same", "same", "external"]),
            &ConfigurationOverride::Inherited,
        )
        .map(|preparation| describe(&preparation))
        .map_err(|error| failure(&error)),
        Scenario::FrozenCohort => check_selection(&prepared(&[]), &CohortSelection::Frozen)
            .map(|()| Cow::Borrowed("checked"))
            .map_err(|error| failure(&error)),
        Scenario::DisabledOutsideCohort => check_selection(
            &prepared(&[("a.py", &["B"][..])]),
            &CohortSelection::Explicit(vec![id("A")]),
        )
        .map(|()| Cow::Borrowed("checked"))
        .map_err(|error| failure(&error)),
        Scenario::UnsupportedContestant => {
            admit_configuration(&request(), RuleConfigurationSupport::Unsupported)
                .map(|()| Cow::Borrowed("admitted"))
                .map_err(|error| failure(&error))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
