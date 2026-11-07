//! The answers the `analysis` fake gives the queries the core hosts: the verdict queries over a scenario's rows and the
//! count and closed queries over a placement's, worked out as those queries say.

use std::collections::BTreeMap;

use crate::domain::applications::types::{AnswerCell, CellText, CellValue, LintAnswer};

/// Whether `rule`, a whole id, is one `only` names: the same id, or the id after a package's `/` or a rule's `#`.
fn named(rule: &str, only: &str) -> bool {
    rule == only || rule.ends_with(&format!("/{only}")) || rule.ends_with(&format!("#{only}"))
}

/// A cell of the verdict's or the count queries' answers, as text.
fn text_cell(name: &str, text: String) -> AnswerCell {
    AnswerCell {
        name: crate::domain::definitions::types::ColumnName(name.into()),
        value: CellValue::Text(CellText(text.into())),
    }
}

/// What ryo answers the verdict queries over `rows`, worked out as they say it: the findings each scenario expects
/// more or fewer of than its check found, of the found ones `only` keeps, the rule it proves that never ran, and each
/// rule `only` names that never ran.
pub(super) fn verdict_answers(
    rows: &crate::domain::scenario_judging::types::VerdictRows,
) -> Vec<LintAnswer> {
    use crate::domain::scenario_judging::constants::{
        EXPECTED_COUNT, EXTRA_RULE, FINDING_PATH, FOUND_COUNT, LINE, MISSING_RULE, ONLY_RULE,
        ONLY_UNRAN_RULE, OWN_UNRAN_RULE, PLACE, RULE,
    };
    type Key = (u32, Box<str>, Box<str>, u32);
    let mut expected: BTreeMap<Key, u32> = BTreeMap::new();
    for row in &rows.expected {
        *expected
            .entry((
                row.scenario.0,
                row.rule.0.clone(),
                row.path.0.clone(),
                row.line.0,
            ))
            .or_default() += 1;
    }
    let mut found: BTreeMap<Key, u32> = BTreeMap::new();
    for row in &rows.found {
        let names = rows
            .only
            .iter()
            .filter(|only| only.scenario == row.scenario)
            .collect::<Vec<_>>();
        if names.is_empty() || names.iter().any(|only| named(&row.full.0, &only.only.0)) {
            *found
                .entry((
                    row.scenario.0,
                    row.rule.0.clone(),
                    row.path.0.clone(),
                    row.line.0,
                ))
                .or_default() += 1;
        }
    }
    let answer = |rule: &str, scenario: u32, cells: Vec<AnswerCell>| LintAnswer {
        rule: crate::domain::applications::types::LintRuleId(rule.into()),
        path: crate::domain::paths::types::RepoPath(scenario.to_string().into()),
        cells,
    };
    let counted = |key: &Key, wanted: u32, got: u32| {
        vec![
            text_cell(RULE, key.1.to_string()),
            text_cell(FINDING_PATH, key.2.to_string()),
            text_cell(LINE, key.3.to_string()),
            text_cell(EXPECTED_COUNT, wanted.to_string()),
            text_cell(FOUND_COUNT, got.to_string()),
        ]
    };
    let mut answers = Vec::new();
    for (key, wanted) in &expected {
        let got = found.get(key).copied().unwrap_or(0);
        if *wanted > got {
            answers.push(answer(MISSING_RULE, key.0, counted(key, *wanted, got)));
        }
    }
    for (key, got) in &found {
        let wanted = expected.get(key).copied().unwrap_or(0);
        if *got > wanted {
            answers.push(answer(EXTRA_RULE, key.0, counted(key, wanted, *got)));
        }
    }
    let ran = |scenario: crate::domain::scenario_judging::types::ScenarioNumber| {
        rows.evaluated
            .iter()
            .filter(move |row| row.scenario == scenario)
    };
    for own in &rows.own {
        if !ran(own.scenario).any(|row| row.rule == own.rule) {
            answers.push(answer(
                OWN_UNRAN_RULE,
                own.scenario.0,
                vec![text_cell(RULE, own.rule.0.to_string())],
            ));
        }
    }
    for only in &rows.only {
        if !ran(only.scenario).any(|row| named(&row.rule.0, &only.only.0)) {
            answers.push(answer(
                ONLY_UNRAN_RULE,
                only.scenario.0,
                vec![
                    text_cell(PLACE, only.place.0.to_string()),
                    text_cell(ONLY_RULE, only.only.0.to_string()),
                ],
            ));
        }
    }
    answers
}

/// What ryo answers the count queries over `rows`, worked out as they say it: each count whose found number is outside
/// the bounds it is held to, with which bounds, and each pair whose count requires any file; an entry whose condition
/// is unanswered, or has no answer, is neither.
pub(super) fn count_answers(rows: &crate::domain::judging::types::CountRows) -> Vec<LintAnswer> {
    use crate::domain::judging::constants::{
        BRANCH, BRANCH_ELSE, BRANCH_OWN, BRANCH_THEN, COUNT_RULE, PAIR_RULE,
    };
    use crate::domain::judging::types::{Bounds, EntryKind};
    use crate::domain::scopes::types::Answer;
    let outside = |found: u32, bounds: Bounds| found < bounds.min.0 || found > bounds.max.0;
    let mut answers = Vec::new();
    for row in &rows.entries {
        let holds = rows
            .conditions
            .iter()
            .find(|answer| answer.scope == row.scope && answer.condition == row.condition)
            .map(|answer| answer.holds);
        let held = if row.condition.0.is_empty() {
            Some((BRANCH_OWN, row.bounds))
        } else {
            match holds {
                Some(Answer::Holds) => Some((BRANCH_THEN, row.then)),
                Some(Answer::Fails) => Some((BRANCH_ELSE, row.otherwise)),
                Some(Answer::Unanswered) | None => None,
            }
        };
        let Some((branch, bounds)) = held else {
            continue;
        };
        let path = crate::domain::paths::types::RepoPath(row.entry.0.to_string().into());
        match row.kind {
            EntryKind::Count if outside(row.found.0, bounds) => answers.push(LintAnswer {
                rule: crate::domain::applications::types::LintRuleId(COUNT_RULE.into()),
                path,
                cells: vec![text_cell(BRANCH, branch.to_string())],
            }),
            EntryKind::Pair if bounds.min.0 > 0 => answers.push(LintAnswer {
                rule: crate::domain::applications::types::LintRuleId(PAIR_RULE.into()),
                path,
                cells: Vec::new(),
            }),
            EntryKind::Count | EntryKind::Pair => {}
        }
    }
    answers.extend(closed_answers(&rows.closings));
    answers
}

/// What the closed query answers over `closings`, decided as it decides: a file beneath a closed directory that no template
/// places, no `tastemaker.toml` is and no nested instance owns is named after the scope that placed files in the nearest directory
/// above it, the last declared of those on a tie, else the last declared of the directory's.
fn closed_answers(closings: &crate::domain::closing::types::Closings) -> Vec<LintAnswer> {
    use crate::domain::closing::constants::{CLOSED_RULE, OWNER};
    closings
        .candidates
        .iter()
        .filter(|candidate| {
            !closings.placed.contains(&candidate.path)
                && !closings.own.contains(&candidate.path)
                && !closings.nested.iter().any(|nested| {
                    nested.group == candidate.group
                        && candidate.path.0.starts_with(&*nested.prefix.0)
                })
        })
        .filter_map(|candidate| {
            let nearest = closings
                .occupied
                .iter()
                .filter(|occupancy| {
                    occupancy.group == candidate.group
                        && candidate.home.0.starts_with(&*occupancy.prefix.0)
                })
                .map(|occupancy| (occupancy.depth, occupancy.owner))
                .max()
                .map(|(_, owner)| owner);
            let last = closings
                .directories
                .iter()
                .find(|directory| directory.group == candidate.group)
                .and_then(|directory| directory.owners.last())
                .map(|owner| owner.scope);
            let owner = nearest.or(last)?;
            Some(LintAnswer {
                rule: crate::domain::applications::types::LintRuleId(CLOSED_RULE.into()),
                path: crate::domain::paths::types::RepoPath(candidate.stray.0.to_string().into()),
                cells: vec![text_cell(OWNER, owner.0.to_string())],
            })
        })
        .collect()
}
