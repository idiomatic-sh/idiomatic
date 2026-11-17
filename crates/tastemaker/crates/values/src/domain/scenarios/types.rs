//! The types of the `scenarios` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! A scenario is one Gherkin Scenario of a rule, a spec or a template: the files it starts from, and the
//! findings its check is expected to report.

use std::collections::BTreeMap;

use crate::domain::entries::types::{Count, ParamName, Value};
use crate::domain::selectors::types::{PathPattern, Selectors};

/// A scenario's id, unique in its definition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScenarioId(pub Box<str>);

/// The Scenario line of a scenario: one sentence saying what the scenario shows.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScenarioLine(pub Box<str>);

/// What a scenario expects of its project: that it has no finding, or that it has the findings the scenario lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Expectation {
    /// `then.findings` is empty or absent: nothing is found.
    Passes,
    /// `then.findings` lists what is found.
    Fails,
}

/// The text of one file of a scenario.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScenarioFileText(pub Box<str>);

/// Whether an expected finding names its line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExpectedLine {
    /// At this line.
    At(crate::domain::findings::types::Line),
    /// About the whole file.
    Whole,
}

/// The id of a rule or check, as a scenario expects it: `#count`, `adapter-test#missing`, `idiomatic.arch/errors`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExpectedRule(pub Box<str>);

/// One finding a scenario expects.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExpectedFinding {
    /// The rule or check; for a rule's own scenario, the rule itself.
    pub rule: ExpectedRuleChoice,
    /// The file.
    pub path: crate::domain::paths::types::RepoPath,
    /// The line.
    pub line: ExpectedLine,
}

/// Whether an expected finding names its rule.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExpectedRuleChoice {
    /// The scenario's own rule.
    Own,
    /// This rule or check.
    Named(ExpectedRule),
}

/// The files of a scenario's fake project, by path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScenarioFiles(pub BTreeMap<crate::domain::paths::types::RepoPath, ScenarioFileText>);

/// A rule's scenario.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleScenario {
    /// Its id.
    pub id: ScenarioId,
    /// The sentence saying what it shows.
    pub scenario: ScenarioLine,
    /// Whether it expects findings.
    pub verdict: Expectation,
    /// Where the scenario places the rule.
    pub include: Selectors,
    /// What the scenario takes out of `include`.
    pub exclude: Selectors,
    /// The values of the rule's params.
    pub params: BTreeMap<ParamName, Value>,
    /// The files of its project.
    pub files: ScenarioFiles,
    /// The rules whose findings are compared when the project is checked: every rule when empty.
    pub only: Vec<ExpectedRule>,
    /// The findings expected.
    pub findings: Vec<ExpectedFinding>,
}

/// A spec's scenario.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecScenario {
    /// Its id.
    pub id: ScenarioId,
    /// The sentence saying what it shows.
    pub scenario: ScenarioLine,
    /// Whether it expects findings.
    pub verdict: Expectation,
    /// The instance's directory within the project; the scenario's files and findings are paths from the project's root.
    pub at: crate::domain::paths::types::RepoPath,
    /// The values of the spec's params.
    pub params: BTreeMap<ParamName, Value>,
    /// The files of its project.
    pub files: ScenarioFiles,
    /// The rules the scenario compares: every rule when empty.
    pub only: Vec<ExpectedRule>,
    /// The findings expected.
    pub findings: Vec<ExpectedFinding>,
}

/// Where a template's scenario places the template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemplateFileChoice {
    /// At this file: a path, or a pattern when many files share the shape.
    Named(PathPattern),
    /// Nowhere named: the scenario brings the `tastemaker.toml` that places the template.
    BroughtWorkspace,
}

/// A template's scenario.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateScenario {
    /// Its id.
    pub id: ScenarioId,
    /// The sentence saying what it shows.
    pub scenario: ScenarioLine,
    /// Whether it expects findings.
    pub verdict: Expectation,
    /// The file the scenario places the template at.
    pub file: TemplateFileChoice,
    /// How many files the scenario expects the template to place.
    pub count: Count,
    /// The values of the template's params.
    pub params: BTreeMap<ParamName, Value>,
    /// The files of its project.
    pub files: ScenarioFiles,
    /// The rules the scenario compares: every rule when empty.
    pub only: Vec<ExpectedRule>,
    /// The findings expected.
    pub findings: Vec<ExpectedFinding>,
}
