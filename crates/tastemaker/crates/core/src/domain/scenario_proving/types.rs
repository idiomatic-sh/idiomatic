//! The types of the `scenario_proving` concept: plain data, public fields and derives, with no constructor and no
//! inherent impl.

use crate::domain::proofs::types::FoundOwn;
use crate::domain::scenario_files::types::ScenarioEntry;
use crate::domain::scenarios::types::{ExpectedFinding, ExpectedRule, ScenarioFiles, ScenarioId};

/// One scenario of a spec or a template, as the project it is checked as.
pub(crate) struct ProjectScenario<'a> {
    /// The scenario's id.
    pub id: &'a ScenarioId,
    /// The rule a rule's scenario proves, whose findings it writes without the rule.
    pub own: Option<FoundOwn<'a>>,
    /// The keys of the entry that places the definition, when the scenario brings no `tastemaker.toml` of its own.
    pub entry: ScenarioEntry,
    /// The files it writes.
    pub files: &'a ScenarioFiles,
    /// The rules whose findings it judges, when it names any.
    pub only: &'a [ExpectedRule],
    /// The findings it expects.
    pub expected: &'a [ExpectedFinding],
}
