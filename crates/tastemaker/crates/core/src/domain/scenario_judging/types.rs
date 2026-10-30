//! The types of the `scenario_judging` concept: plain data, public fields and derives, with no constructor and no
//! inherent impl. The rows tm hosts for the verdict queries, and the rows they answer.

/// The number a scenario has among the scenarios one verdict call judges, from zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScenarioNumber(pub u32);

/// The number a finding has among the findings of the scenarios one verdict call judges: rows are sets, so two equal
/// findings are two rows only by it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FindingNumber(pub u32);

/// The place a rule has in a scenario's `only`, from zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OnlyPlace(pub u32);

/// A rule's id as the verdict compares it: blank for the rule a scenario proves, else, for a found finding, the expected
/// rule the found one ends with, else the whole id.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VerdictRule(pub Box<str>);

/// How many findings of one rule, path and line a scenario expects, or its check found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FindingCount(pub u32);

/// A row of `tastemaker.expected`: a finding a scenario expects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedRow {
    /// The scenario.
    pub scenario: ScenarioNumber,
    /// The finding.
    pub finding: FindingNumber,
    /// The rule, blank for the scenario's own.
    pub rule: VerdictRule,
    /// The path, `.` for the project's root.
    pub path: crate::domain::paths::types::RepoPath,
    /// The line, from one; zero for the whole file.
    pub line: crate::domain::findings::types::Line,
}

/// A row of `tastemaker.found`: a finding a scenario's check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundRow {
    /// The scenario.
    pub scenario: ScenarioNumber,
    /// The finding.
    pub finding: FindingNumber,
    /// The rule as it compares with the expected ones.
    pub rule: VerdictRule,
    /// The rule's whole id, as `only` names it.
    pub full: VerdictRule,
    /// The path, `.` for the project's root.
    pub path: crate::domain::paths::types::RepoPath,
    /// The line, from one; zero for the whole file.
    pub line: crate::domain::findings::types::Line,
}

/// A row of `tastemaker.only`: a rule a scenario's `only` names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OnlyRow {
    /// The scenario.
    pub scenario: ScenarioNumber,
    /// Its place in `only`.
    pub place: OnlyPlace,
    /// The rule, as `only` writes it.
    pub only: crate::domain::scenarios::types::ExpectedRule,
}

/// A row of `tastemaker.own` or `tastemaker.evaluated`: the rule a scenario proves, or a rule or check its check ran.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuleRow {
    /// The scenario.
    pub scenario: ScenarioNumber,
    /// The rule's whole id.
    pub rule: VerdictRule,
}

/// What the scenarios of one verdict call expected, found, named in `only`, proved and ran: the rows tm hosts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VerdictRows {
    /// `tastemaker.expected`.
    pub expected: Vec<ExpectedRow>,
    /// `tastemaker.found`.
    pub found: Vec<FoundRow>,
    /// `tastemaker.only`.
    pub only: Vec<OnlyRow>,
    /// `tastemaker.own`.
    pub own: Vec<RuleRow>,
    /// `tastemaker.evaluated`.
    pub evaluated: Vec<RuleRow>,
}

/// One rule, path and line whose findings a scenario expected more or fewer of than its check found: how many each.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScenarioCounts {
    /// The scenario.
    pub scenario: ScenarioNumber,
    /// The rule.
    pub rule: VerdictRule,
    /// The path.
    pub path: crate::domain::paths::types::RepoPath,
    /// The line; zero for the whole file.
    pub line: crate::domain::findings::types::Line,
    /// How many the scenario expects.
    pub expected: FindingCount,
    /// How many its check found.
    pub found: FindingCount,
}

/// A row the verdict queries answered.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScenarioVerdict {
    /// Expected more than found.
    Missing(ScenarioCounts),
    /// Found more than expected.
    Extra(ScenarioCounts),
    /// The rule the scenario proves never ran.
    OwnUnran(RuleRow),
    /// A rule `only` names never ran.
    OnlyUnran(OnlyRow),
}
