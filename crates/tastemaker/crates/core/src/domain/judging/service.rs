//! The `judging` service: free functions named by verb that turn the counts and pairs a placement left waiting, and what
//! the conditions they read answered, into the rows tm hosts for the count queries, and the rows those queries answer
//! into findings, with no IO and no ports. Which counts are refused and which pairs stand is the queries'
//! (`super::constants`); what a finding says is written here.

use std::collections::{BTreeMap, BTreeSet};

use super::constants::{
    AT_ROOT, BRANCH, BRANCH_ELSE, BRANCH_OWN, BRANCH_THEN, COUNT_QUERY, COUNT_RULE, DIRECTORIES,
    FILES, ONE_DIRECTORY, PAIR_QUERY, PAIR_RULE, VALUES,
};
use super::types::{
    Bounds, BoundsHeld, ConditionRow, CountRows, EntryKind, EntryNumber, EntryRow, Judgment,
};
use crate::domain::applications::types::{
    CellValue, GlobText, LintAnswer, LintRuleId, Plan, PlannedRule, Purpose,
};
use crate::domain::closing::constants::{CLOSED_QUERY, CLOSED_RULE, OWNER};
use crate::domain::closing::types::{Closings, Stray, StrayNumber};
use crate::domain::entries::service::{answered, count_text};
use crate::domain::entries::types::{Amount, Bound, Branch, ConditionName, Count};
use crate::domain::findings::service::error;
use crate::domain::findings::types::Finding;
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::types::{
    Answer, ConditionAt, CountCheck, CountMessage, CountUnit, Identity, PairCheck, ScopeIndex,
};

/// The rules of a count call: its three queries, over every relation it hosts and no file.
#[must_use]
pub fn rules() -> Vec<PlannedRule> {
    [
        (COUNT_RULE, COUNT_QUERY),
        (PAIR_RULE, PAIR_QUERY),
        (CLOSED_RULE, CLOSED_QUERY),
    ]
    .into_iter()
    .map(|(id, query)| PlannedRule {
        id: LintRuleId(id.into()),
        query: crate::domain::definitions::types::QueryText(query.into()),
        parameters: BTreeMap::new(),
        include: vec![GlobText(
            crate::domain::applications::constants::EVERYTHING.into(),
        )],
        exclude: Vec::new(),
    })
    .collect()
}

/// What each condition of `plan` answered in the lint call that ran its rules, `answers`: it holds when its rule
/// selected a row, it is unanswered when its rule was `refused`, and it fails otherwise.
#[must_use]
pub fn holds(
    plan: &Plan,
    refused: &BTreeSet<LintRuleId>,
    answers: &[LintAnswer],
) -> BTreeMap<ConditionAt, Answer> {
    let mut holds = BTreeMap::new();
    for (id, purpose) in &plan.purposes {
        if let Purpose::Condition { scope, name, .. } = purpose {
            let answer = if refused.contains(id) {
                Answer::Unanswered
            } else {
                Answer::Fails
            };
            let _: Option<Answer> = holds.insert(
                ConditionAt {
                    scope: *scope,
                    condition: name.clone(),
                },
                answer,
            );
        }
    }
    for found in answers {
        if let Some(Purpose::Condition { scope, name, .. }) = plan.purposes.get(&found.rule) {
            let _: Option<Answer> = holds.insert(
                ConditionAt {
                    scope: *scope,
                    condition: name.clone(),
                },
                Answer::Holds,
            );
        }
    }
    holds
}

/// The row a check hosts for one count or pair waiting, numbered `entry`: the condition its `count` reads (blank for
/// none), its own bounds, and those of each branch; a bound a count does not have is none to the fewest and every
/// number to the most.
fn entry_row(
    entry: EntryNumber,
    kind: EntryKind,
    scope: ScopeIndex,
    identity: Identity,
    found: Amount,
    count: &Count,
) -> EntryRow {
    let any = Bounds {
        min: Amount(0),
        max: Amount(u32::MAX),
    };
    let exactly = |amount: Amount| Bounds {
        min: amount,
        max: amount,
    };
    let mut row = EntryRow {
        entry,
        scope,
        identity,
        kind,
        found,
        condition: ConditionName(Box::default()),
        bounds: any,
        then: any,
        otherwise: any,
    };
    match count {
        Count::Exactly(amount) => row.bounds = exactly(*amount),
        Count::Range { min, max } => {
            row.bounds = Bounds {
                min: *min,
                max: match max {
                    Bound::AtMost(most) => *most,
                    Bound::Unbounded => Amount(u32::MAX),
                },
            };
        }
        Count::When {
            condition,
            then,
            otherwise,
        } => {
            row.condition = condition.clone();
            row.then = exactly(*then);
            row.otherwise = exactly(*otherwise);
        }
    }
    row
}

/// The entry a count's message names.
fn identity_of(message: &CountMessage) -> &Identity {
    match message {
        CountMessage::Spec { identity }
        | CountMessage::Template { identity, .. }
        | CountMessage::Grouped { identity, .. } => identity,
    }
}

/// The rows a check hosts for the count queries: each of `counts`, numbered from zero, then each of `pairs`, numbered
/// after them, with its bounds; and what each condition they read answered, `holds`, a condition no rule answered
/// failing; and, for the closed query, what the closed directories hold, `closings`.
#[must_use]
pub fn host(
    counts: &[CountCheck],
    pairs: &[PairCheck],
    holds: &BTreeMap<ConditionAt, Answer>,
    closings: &Closings,
) -> CountRows {
    let mut rows = CountRows {
        closings: closings.clone(),
        ..CountRows::default()
    };
    let mut numbers = (0..).map(EntryNumber);
    for (check, entry) in counts.iter().zip(numbers.by_ref()) {
        rows.entries.push(entry_row(
            entry,
            EntryKind::Count,
            check.scope,
            identity_of(&check.message).clone(),
            check.found,
            &check.count,
        ));
    }
    for (pair, entry) in pairs.iter().zip(numbers) {
        rows.entries.push(entry_row(
            entry,
            EntryKind::Pair,
            pair.scope,
            Identity(Box::default()),
            Amount(0),
            &pair.count,
        ));
    }
    let asked = rows
        .entries
        .iter()
        .filter(|row| !row.condition.0.is_empty())
        .map(|row| ConditionAt {
            scope: row.scope,
            condition: row.condition.clone(),
        })
        .collect::<BTreeSet<_>>();
    for at in asked {
        let answer = holds.get(&at).copied().unwrap_or(Answer::Fails);
        rows.conditions.push(ConditionRow {
            scope: at.scope,
            condition: at.condition,
            holds: answer,
        });
    }
    rows
}

/// Which bounds the count query held a refused count to, by the `branch` cell of `answer`; none when it holds no
/// branch the query writes. A cell the same text as the subject comes back as a file.
fn held_of(answer: &LintAnswer) -> Option<BoundsHeld> {
    let cell = answer.cells.iter().find(|cell| &*cell.name.0 == BRANCH)?;
    let written = match &cell.value {
        CellValue::File(path) => &path.0,
        CellValue::Text(text) => &text.0,
        CellValue::Span(_) => return None,
    };
    match written.parse() {
        Ok(BRANCH_OWN) => Some(BoundsHeld::Own),
        Ok(BRANCH_THEN) => Some(BoundsHeld::Then),
        Ok(BRANCH_ELSE) => Some(BoundsHeld::Otherwise),
        Ok(_) | Err(_) => None,
    }
}

/// What one row a count query answered says of the count or pair its subject numbers; none for a row of another query,
/// or one whose subject or cells are no number the queries write.
#[must_use]
pub fn judgment_of(answer: &LintAnswer) -> Option<Judgment> {
    let entry = match answer.path.0.parse() {
        Ok(number) => EntryNumber(number),
        Err(_) => return None,
    };
    match &*answer.rule.0 {
        COUNT_RULE => Some(Judgment::Refused {
            entry,
            held: held_of(answer)?,
        }),
        PAIR_RULE => Some(Judgment::Missing { entry }),
        CLOSED_RULE => stray_of(answer).map(Judgment::Stray),
        _ => None,
    }
}

/// The scope the closed query named for a stray of `answer`; none for a row whose owner is no number the query writes.
fn stray_of(answer: &LintAnswer) -> Option<Stray> {
    let stray = match answer.path.0.parse() {
        Ok(number) => StrayNumber(number),
        Err(_) => return None,
    };
    let cell = answer.cells.iter().find(|cell| &*cell.name.0 == OWNER)?;
    let written = match &cell.value {
        CellValue::File(path) => &path.0,
        CellValue::Text(text) => &text.0,
        CellValue::Span(_) => return None,
    };
    match written.parse() {
        Ok(owner) => Some(Stray {
            stray,
            owner: ScopeIndex(owner),
        }),
        Err(_) => None,
    }
}

/// The count `check` was held to, `held`: its own, or the branch its condition's answer picked.
fn held_count(count: &Count, held: BoundsHeld) -> Count {
    match held {
        BoundsHeld::Own => count.clone(),
        BoundsHeld::Then => answered(count, Branch::Then),
        BoundsHeld::Otherwise => answered(count, Branch::Otherwise),
    }
}

/// What the refusal of `check`, held to `count`, says.
fn refusal(check: &CountCheck, count: &Count) -> Finding {
    let written = count_text(count);
    let found = check.found.0;
    let message = match &check.message {
        CountMessage::Spec { identity } => format!(
            "`{}` applies at {found} director{}, but its count requires {written}",
            identity.0,
            if found == 1 {
                ONE_DIRECTORY
            } else {
                DIRECTORIES
            }
        ),
        CountMessage::Template {
            identity,
            file,
            unit,
            directory,
        } => format!(
            "`{file}` matches {found} {} {}, but `{}` requires {written}",
            match unit {
                CountUnit::Files => FILES,
                CountUnit::Values => VALUES,
            },
            if directory.0.is_empty() {
                AT_ROOT.to_owned()
            } else {
                format!("in `{}`", directory.0)
            },
            identity.0
        ),
        CountMessage::Grouped { identity, captures } => format!(
            "`{}` matches {found} value(s) for {captures}, but its count requires {written} for each",
            identity.0
        ),
    };
    error(check.rule.clone(), check.path.clone(), message)
}

/// The findings of the counts and pairs a check hosted, `counts` and `pairs` as `host` numbered them, from what the
/// count queries answered of them, `judgments`: each refused count's in the order of the counts, then the findings of
/// each pair that stands, in the order of the pairs, then those of the strays of the closed directories `closings`,
/// `excluded` being the files under a `rules-exclude`.
#[must_use]
pub fn findings(
    counts: &[CountCheck],
    pairs: &[PairCheck],
    closings: &Closings,
    excluded: &BTreeSet<RepoPath>,
    judgments: &[Judgment],
) -> Vec<Finding> {
    let mut refused = BTreeMap::new();
    let mut standing = BTreeSet::new();
    let mut strays = Vec::new();
    for judgment in judgments {
        match *judgment {
            Judgment::Refused { entry, held } => {
                let _: Option<BoundsHeld> = refused.insert(entry, held);
            }
            Judgment::Missing { entry } => {
                let _: bool = standing.insert(entry);
            }
            Judgment::Stray(stray) => strays.push(stray),
        }
    }
    let mut numbers = (0..).map(EntryNumber);
    let mut found = Vec::new();
    for (check, entry) in counts.iter().zip(numbers.by_ref()) {
        if let Some(held) = refused.get(&entry) {
            found.push(refusal(check, &held_count(&check.count, *held)));
        }
    }
    for (pair, entry) in pairs.iter().zip(numbers) {
        if standing.contains(&entry) {
            found.extend(pair.findings.iter().cloned());
        }
    }
    found.extend(crate::domain::closing::service::findings(
        closings, excluded, &strays,
    ));
    found
}
