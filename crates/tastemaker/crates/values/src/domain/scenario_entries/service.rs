//! The `scenario_entries` service: free functions named by verb that say the keys of the entry that places what a
//! scenario proves, with no IO and no ports. The manifest port writes them.

use std::collections::BTreeMap;

use crate::domain::entries::service::derived;
use crate::domain::scenario_files::constants::{EVERY_PATH, HERE};
use crate::domain::scenario_files::types::{CountKey, FileKey, ScenarioEntry, ScenarioKeys};
use crate::domain::scenarios::types::{
    RuleScenario, SpecScenario, TemplateFileChoice, TemplateScenario,
};
use crate::domain::selectors::types::{PathPattern, Selector, Selectors};

/// The keys of an entry placed at `include` and nothing more.
fn placed_at(include: Selectors) -> ScenarioKeys {
    ScenarioKeys {
        include,
        exclude: Selectors::default(),
        file: FileKey::Unnamed,
        count: CountKey::Derived,
        params: BTreeMap::new(),
    }
}

/// One path, as a selector list.
fn only(path: PathPattern) -> Selectors {
    Selectors(vec![Selector::Glob(path)])
}

/// The keys of the entry that places a spec once, at the project's root: a wrapper's.
#[must_use]
pub fn root_entry() -> ScenarioKeys {
    placed_at(only(PathPattern(HERE.into())))
}

/// The keys of the entry that places the spec of a scenario at its `at` with its params.
#[must_use]
pub fn spec_entry(scenario: &SpecScenario) -> ScenarioKeys {
    let at = if scenario.at.0.is_empty() {
        HERE
    } else {
        &scenario.at.0
    };
    ScenarioKeys {
        params: scenario.params.clone(),
        ..placed_at(only(PathPattern(at.into())))
    }
}

/// The keys of the entry that places the rule of a scenario where its `include` and `exclude` say, with its params. A
/// scenario that names no `include` places the rule over every file.
#[must_use]
pub fn rule_entry(scenario: &RuleScenario) -> ScenarioKeys {
    let include = if scenario.include.0.is_empty() {
        only(PathPattern(EVERY_PATH.into()))
    } else {
        scenario.include.clone()
    };
    ScenarioKeys {
        exclude: scenario.exclude.clone(),
        params: scenario.params.clone(),
        ..placed_at(include)
    }
}

/// The keys of the entry that places the template of a scenario at its `file`, as many times as its `count` says, with
/// its params; none when the scenario names no `file`.
#[must_use]
pub fn template_entry(scenario: &TemplateScenario) -> ScenarioEntry {
    match &scenario.file {
        TemplateFileChoice::Named(file) => ScenarioEntry::Keys(ScenarioKeys {
            file: FileKey::Named(file.clone()),
            // A count is written only where it differs from what the file's path says.
            count: if scenario.count == derived(&[Some(file)], &std::collections::BTreeSet::new()) {
                CountKey::Derived
            } else {
                CountKey::Written(scenario.count.clone())
            },
            params: scenario.params.clone(),
            ..placed_at(Selectors::default())
        }),
        TemplateFileChoice::BroughtWorkspace => ScenarioEntry::Brought,
    }
}
