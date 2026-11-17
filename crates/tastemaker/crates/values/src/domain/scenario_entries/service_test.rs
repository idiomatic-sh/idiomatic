//! What the `scenario_entries` service says the entry that places what a scenario proves holds, on values alone.

use std::collections::BTreeMap;

use super::service::{rule_entry, template_entry};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::entries::types::{Amount, Count};
use crate::domain::scenario_files::types::{CountKey, FileKey, ScenarioEntry};
use crate::domain::scenarios::types::{
    Expectation, RuleScenario, ScenarioFiles, ScenarioId, ScenarioLine, TemplateFileChoice,
    TemplateScenario,
};
use crate::domain::selectors::types::{PathPattern, RegexText, Selector, Selectors};

/// What a row writes the entry of.
#[derive(Clone, Copy)]
enum Writing {
    /// A rule's scenario whose `include` names a glob and a regex, and whose `exclude` a glob.
    Rule,
    /// A rule's scenario that names no `include`: it places the rule over every file.
    RuleEverywhere,
    /// A template's scenario that names the file it places the template at.
    TemplateAtAFile,
    /// A template's scenario that brings the `tastemaker.toml` that places it.
    TemplateBringingItsWorkspace,
}

type Input = Writing;
/// What the entry's keys hold, in debug form, where they differ from what they should.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_scenario_is_placed_by_the_selectors_its_include_and_exclude_name"),
        word: Word::Succeeds,
        input: || Writing::Rule,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rule_scenario_without_an_include_is_placed_over_every_file"),
        word: Word::Succeeds,
        input: || Writing::RuleEverywhere,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_template_scenario_is_placed_at_the_file_it_names"),
        word: Word::Succeeds,
        input: || Writing::TemplateAtAFile,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_template_scenario_that_brings_its_workspace_is_placed_by_it"),
        word: Word::Succeeds,
        input: || Writing::TemplateBringingItsWorkspace,
        expect: Expect::Value(String::new()),
    },
];

fn rule(include: Selectors, exclude: Selectors) -> RuleScenario {
    RuleScenario {
        id: ScenarioId("c".into()),
        scenario: ScenarioLine("A scenario places the rule.".into()),
        verdict: Expectation::Passes,
        include,
        exclude,
        params: BTreeMap::new(),
        files: ScenarioFiles::default(),
        only: Vec::new(),
        findings: Vec::new(),
    }
}

fn template(file: TemplateFileChoice) -> TemplateScenario {
    TemplateScenario {
        id: ScenarioId("c".into()),
        scenario: ScenarioLine("A scenario places the template.".into()),
        verdict: Expectation::Passes,
        file,
        count: Count::Exactly(Amount(2)),
        params: BTreeMap::new(),
        files: ScenarioFiles::default(),
        only: Vec::new(),
        findings: Vec::new(),
    }
}

/// What the row gave, and what it should give.
fn said(writing: Writing) -> (String, String) {
    match writing {
        Writing::Rule => {
            let include = Selectors(vec![
                Selector::Glob(PathPattern("src/**".into())),
                Selector::Regex(RegexText("x".into())),
            ]);
            let exclude = Selectors(vec![Selector::Glob(PathPattern("src/gen/**".into()))]);
            let keys = rule_entry(&rule(include.clone(), exclude.clone()));
            (
                format!("{:?} {:?}", keys.include, keys.exclude),
                format!("{include:?} {exclude:?}"),
            )
        }
        Writing::RuleEverywhere => (
            format!(
                "{:?}",
                rule_entry(&rule(Selectors::default(), Selectors::default())).include
            ),
            format!(
                "{:?}",
                Selectors(vec![Selector::Glob(PathPattern("**".into()))])
            ),
        ),
        Writing::TemplateAtAFile => (
            match template_entry(&template(TemplateFileChoice::Named(PathPattern(
                "a.txt".into(),
            )))) {
                ScenarioEntry::Keys(keys) => format!("{:?} {:?}", keys.file, keys.count),
                ScenarioEntry::Brought => String::new(),
            },
            format!(
                "{:?} {:?}",
                FileKey::Named(PathPattern("a.txt".into())),
                CountKey::Written(Count::Exactly(Amount(2)))
            ),
        ),
        Writing::TemplateBringingItsWorkspace => (
            format!(
                "{:?}",
                template_entry(&template(TemplateFileChoice::BroughtWorkspace))
            ),
            "Brought".to_owned(),
        ),
    }
}

fn subject(writing: Input) -> Value {
    let (said, wanted) = said(writing);
    if said == wanted { String::new() } else { said }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |writing| Ok(subject(writing)));
}
