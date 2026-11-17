//! The `scenario_judging` service: free functions named by verb that turn what a scenario's checked project found, and
//! what the scenario expects, into the rows tm hosts for the verdict queries, and the rows they answer into problems.
//! The verdict itself (what is missing, what is extra, what never ran) is the queries' (`super::constants`).

use std::collections::BTreeSet;
use std::ops::Not as _;

use super::constants::{
    EXPECTED_COUNT, EXTRA_QUERY, EXTRA_RULE, FINDING_PATH, FOUND_COUNT, LINE, MISSING_QUERY,
    MISSING_RULE, ONLY_RULE, ONLY_UNRAN_QUERY, ONLY_UNRAN_RULE, OWN_UNRAN_QUERY, OWN_UNRAN_RULE,
    PLACE, RULE,
};
use super::types::{
    ExpectedRow, FindingCount, FindingNumber, FoundRow, OnlyPlace, OnlyRow, RuleRow,
    ScenarioCounts, ScenarioNumber, ScenarioVerdict, VerdictRows, VerdictRule,
};
use crate::domain::applications::types::{
    CellText, CellValue, GlobText, LintAnswer, LintRuleId, PlannedRule,
};
use crate::domain::definitions::types::ColumnName;
use crate::domain::findings::service::printed;
use crate::domain::findings::types::{Finding, FindingRule, Line, MessageText, Place};
use crate::domain::paths::types::RepoPath;
use crate::domain::proofs::types::{FoundOwn, Problem};
use crate::domain::scenario_files::constants::{HERE, OUTDATED_CHECK};
use crate::domain::scenarios::types::{
    ExpectedFinding, ExpectedLine, ExpectedRule, ExpectedRuleChoice, ScenarioId,
};

/// The rules of a verdict call: its four queries, over every relation it hosts and no file.
#[must_use]
pub fn rules() -> Vec<PlannedRule> {
    [
        (MISSING_RULE, MISSING_QUERY),
        (EXTRA_RULE, EXTRA_QUERY),
        (OWN_UNRAN_RULE, OWN_UNRAN_QUERY),
        (ONLY_UNRAN_RULE, ONLY_UNRAN_QUERY),
    ]
    .into_iter()
    .map(|(id, query)| PlannedRule {
        id: LintRuleId(id.into()),
        query: crate::domain::definitions::types::QueryText(query.into()),
        parameters: std::collections::BTreeMap::new(),
        include: vec![GlobText(
            crate::domain::applications::constants::EVERYTHING.into(),
        )],
        exclude: Vec::new(),
    })
    .collect()
}

/// The scenario a row the verdict answered is of.
#[must_use]
pub const fn scenario_of(verdict: &ScenarioVerdict) -> ScenarioNumber {
    match verdict {
        ScenarioVerdict::Missing(counted) | ScenarioVerdict::Extra(counted) => counted.scenario,
        ScenarioVerdict::OwnUnran(own) => own.scenario,
        ScenarioVerdict::OnlyUnran(only) => only.scenario,
    }
}

/// The next finding number of `rows`: one more than every finding it holds.
fn next(rows: &VerdictRows) -> FindingNumber {
    FindingNumber(u32::try_from(rows.expected.len() + rows.found.len()).unwrap_or(u32::MAX))
}

/// A rule's id as the verdict compares an expected one: blank for the scenario's own rule, `own`.
fn expected_rule(finding: &ExpectedFinding, own: Option<FoundOwn<'_>>) -> VerdictRule {
    VerdictRule(match &finding.rule {
        ExpectedRuleChoice::Own => Box::default(),
        ExpectedRuleChoice::Named(rule) if own.is_some_and(|own| *rule.0 == *own.0) => {
            Box::default()
        }
        ExpectedRuleChoice::Named(rule) => rule.0.clone(),
    })
}

/// A found rule's id as the verdict compares it: blank when it is the scenario's own, `own`; else the first expected
/// rule it ends with, so `#count` matches `idiomatic.arch/errors#count`; else the whole id.
fn found_rule(
    rule: &VerdictRule,
    own: Option<FoundOwn<'_>>,
    expectations: &[ExpectedFinding],
) -> VerdictRule {
    if own.is_some_and(|own| &*rule.0 == own.0) {
        return VerdictRule::default();
    }
    VerdictRule(
        expectations
            .iter()
            .find_map(|expectation| match &expectation.rule {
                ExpectedRuleChoice::Named(expected) if rule.0.ends_with(&*expected.0) => {
                    Some(expected.0.clone())
                }
                ExpectedRuleChoice::Named(_) | ExpectedRuleChoice::Own => None,
            })
            .unwrap_or_else(|| rule.0.clone()),
    )
}

/// Adds to `rows` what scenario `scenario` hosts for the verdict: the findings it expects, the findings its check
/// reported (`#outdated` left out), each at the line its place starts on and its rule named as the
/// verdict compares it, the rules its `only` names, the rule it proves, `own`, and every rule and check its check
/// `evaluated`. Which found findings `only` keeps, and what never ran, the queries decide.
pub fn host(
    scenario: ScenarioNumber,
    own: Option<FoundOwn<'_>>,
    expectations: &[ExpectedFinding],
    only: &[ExpectedRule],
    reported: &[Finding],
    evaluated: &BTreeSet<FindingRule>,
    rows: &mut VerdictRows,
) {
    for expectation in expectations {
        let finding = next(rows);
        rows.expected.push(ExpectedRow {
            scenario,
            finding,
            rule: expected_rule(expectation, own),
            path: expectation.path.clone(),
            line: match expectation.line {
                ExpectedLine::At(line) => line,
                ExpectedLine::Whole => Line(0),
            },
        });
    }
    for finding in reported
        .iter()
        .filter(|finding| printed(&finding.rule).0.ends_with(OUTDATED_CHECK).not())
    {
        let full = VerdictRule(printed(&finding.rule).0);
        let number = next(rows);
        rows.found.push(FoundRow {
            scenario,
            finding: number,
            rule: found_rule(&full, own, expectations),
            full,
            // A finding about the project's root directory is at `.`, as a scenario writes it.
            path: if finding.path.0.is_empty() {
                RepoPath(HERE.into())
            } else {
                finding.path.clone()
            },
            line: match finding.place {
                Place::At(region) => region.start_line,
                Place::Line(line) => line,
                Place::Whole => Line(0),
            },
        });
    }
    for (place, rule) in only.iter().enumerate() {
        rows.only.push(OnlyRow {
            scenario,
            place: OnlyPlace(u32::try_from(place).unwrap_or(u32::MAX)),
            only: rule.clone(),
        });
    }
    if let Some(FoundOwn(rule)) = own {
        rows.own.push(RuleRow {
            scenario,
            rule: VerdictRule(rule.into()),
        });
    }
    for ran in evaluated {
        rows.evaluated.push(RuleRow {
            scenario,
            rule: VerdictRule(printed(ran).0),
        });
    }
}

/// The text of a cell, whatever the lint call made of it: a file, a span or text.
fn text_of(value: &CellValue) -> CellText {
    match value {
        CellValue::File(path) => CellText(path.0.clone()),
        CellValue::Span(span) => CellText(format!("{}..{}", span.start.0, span.end.0).into()),
        CellValue::Text(text) => text.clone(),
    }
}

/// A number a query wrote in the cell `name` of `answer`, by its text; none when it is no number.
fn number_of<N>(answer: &LintAnswer, name: &ColumnName) -> Option<N>
where
    N: std::str::FromStr,
{
    // ignore: idiomatic.arch/no-ok-on-result - a cell that is no number is no row of the verdict queries, which the caller says reads not
    cell_of(answer, name)?.0.parse().ok()
}

/// The text of the cell `name` of `answer`.
fn cell_of(answer: &LintAnswer, name: &ColumnName) -> Option<CellText> {
    answer
        .cells
        .iter()
        .find(|cell| cell.name == *name)
        .map(|cell| text_of(&cell.value))
}

/// What one row a verdict query answered says, for the scenario its subject numbers; none for a row of another query,
/// or one whose cells do not read.
#[must_use]
pub fn verdict_of(answer: &LintAnswer) -> Option<ScenarioVerdict> {
    let scenario = match answer.path.0.parse() {
        Ok(number) => ScenarioNumber(number),
        Err(_) => return None,
    };
    let column = |name| ColumnName(From::from(name));
    let counted = || {
        Some(ScenarioCounts {
            scenario,
            rule: VerdictRule(cell_of(answer, &column(RULE))?.0),
            path: RepoPath(cell_of(answer, &column(FINDING_PATH))?.0),
            line: Line(number_of(answer, &column(LINE))?),
            expected: FindingCount(number_of(answer, &column(EXPECTED_COUNT))?),
            found: FindingCount(number_of(answer, &column(FOUND_COUNT))?),
        })
    };
    match &*answer.rule.0 {
        MISSING_RULE => counted().map(ScenarioVerdict::Missing),
        EXTRA_RULE => counted().map(ScenarioVerdict::Extra),
        OWN_UNRAN_RULE => Some(ScenarioVerdict::OwnUnran(RuleRow {
            scenario,
            rule: VerdictRule(cell_of(answer, &column(RULE))?.0),
        })),
        ONLY_UNRAN_RULE => Some(ScenarioVerdict::OnlyUnran(OnlyRow {
            scenario,
            place: OnlyPlace(number_of(answer, &column(PLACE))?),
            only: ExpectedRule(cell_of(answer, &column(ONLY_RULE))?.0),
        })),
        _ => None,
    }
}

/// What a finding of the verdict reads as: `rule path:line`, or `rule path` for a whole file; the rule left out for the
/// scenario's own, so the text starts with a space then.
fn observed(counted: &ScenarioCounts) -> MessageText {
    MessageText(
        if counted.line.0 == 0 {
            format!("{} {}", counted.rule.0, counted.path.0)
        } else {
            format!("{} {}:{}", counted.rule.0, counted.path.0, counted.line.0)
        }
        .into(),
    )
}

/// The problems of the scenario `name` from what the verdict queries answered of it, `verdicts`: the rule under proof
/// that reached no file of the project, each rule `only` names that the check did not evaluate, in `only`'s order,
/// then each finding expected and not found, then each found and not expected, each list in the order of its text.
#[must_use]
pub fn problems(name: &ScenarioId, verdicts: &[&ScenarioVerdict]) -> Vec<Problem> {
    let mut problems = Vec::new();
    for verdict in verdicts {
        if let ScenarioVerdict::OwnUnran(own) = verdict {
            problems.push(Problem(
                format!(
                    "scenario `{}`: `{}` reached no file of the project, so the scenario proves nothing",
                    name.0, own.rule.0
                )
                .into(),
            ));
        }
    }
    let mut unran = verdicts
        .iter()
        .filter_map(|verdict| match verdict {
            ScenarioVerdict::OnlyUnran(only) => Some(only),
            ScenarioVerdict::Missing(_)
            | ScenarioVerdict::Extra(_)
            | ScenarioVerdict::OwnUnran(_) => None,
        })
        .collect::<Vec<_>>();
    unran.sort_by_key(|only| only.place);
    for only in unran {
        problems.push(Problem(
            format!(
                "scenario `{}`: `only` names `{}`, which the check did not evaluate",
                name.0, only.only.0
            )
            .into(),
        ));
    }
    let mut missing = Vec::new();
    let mut extra = Vec::new();
    for verdict in verdicts {
        match verdict {
            ScenarioVerdict::Missing(counted) => {
                let times = counted.expected.0.saturating_sub(counted.found.0);
                missing.extend(std::iter::repeat_n(
                    observed(counted),
                    usize::try_from(times).unwrap_or(0),
                ));
            }
            ScenarioVerdict::Extra(counted) => {
                let times = counted.found.0.saturating_sub(counted.expected.0);
                extra.extend(std::iter::repeat_n(
                    observed(counted),
                    usize::try_from(times).unwrap_or(0),
                ));
            }
            ScenarioVerdict::OwnUnran(_) | ScenarioVerdict::OnlyUnran(_) => {}
        }
    }
    missing.sort();
    extra.sort();
    for item in missing {
        problems.push(Problem(
            format!(
                "scenario `{}`: expected {}, not found",
                name.0,
                item.0.trim()
            )
            .into(),
        ));
    }
    for item in extra {
        problems.push(Problem(
            format!(
                "scenario `{}`: found {}, not expected",
                name.0,
                item.0.trim()
            )
            .into(),
        ));
    }
    problems
}
