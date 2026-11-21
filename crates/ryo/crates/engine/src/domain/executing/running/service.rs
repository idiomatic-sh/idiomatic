//! The services of the `running` part: the walk of everything a rule can reach in the code (its branches' programs, the
//! bodies and patterns they ask, the tables they read and the tables' fills, the tables a need may make), asking of each
//! word and fill whether the executor runs it. The load refuses a rule that can reach one it does not, so no such rule
//! reaches a scan. Each `Op` is named: a code that is newly built must be marked as running here to compile. A pipeline's
//! words and a table's fill are the `filling` part's; what both kinds of word share is the `places` part's.

use super::filling::service::{fill, stage_word};
use super::places::service::{body, site_table, tested};
use super::types::{WalkPlace, WordBuilt};
use crate::domain::code::anchoring::service::{
    each_match_filled_many_of, each_match_filled_of, each_match_made_of, each_match_node_of,
    each_row_made_of, latest_of,
};
use crate::domain::code::controlling::service::{exists_of, fork_of, or_of};
use crate::domain::code::deciding::service::test_of;
use crate::domain::code::small_codes::service::comparison_of;
use crate::domain::code::types::{CodeEntry, CodeWord, Op, ProgramCode, RuleCode, WordSlot};
use crate::domain::code::walking::service::nearest_of;
use crate::domain::made_tables::reading::service::latest_site;
use crate::domain::needs::service::tables_asked;
use crate::domain::programs::operands::service::listed;

/// Whether the loop runs a word of a step program, and what the word reaches.
fn step_word(
    code: &ProgramCode,
    word: CodeWord,
    unit: WordSlot,
    out: &mut Vec<WalkPlace>,
) -> WordBuilt {
    match word.op {
        Op::EachMatchNode => {
            site_table(code, each_match_node_of(word).index, out);
            WordBuilt::Runs
        }
        Op::EachMatchFilled => {
            site_table(code, each_match_filled_of(word).index, out);
            WordBuilt::Runs
        }
        Op::EachMatchFilledMany => {
            site_table(code, each_match_filled_many_of(word).index, out);
            WordBuilt::Runs
        }
        Op::EachMatchMade => {
            site_table(code, each_match_made_of(word).index, out);
            WordBuilt::Runs
        }
        Op::End
        | Op::EachKind
        | Op::EachRowNode
        | Op::EachRowFilled
        | Op::ReadNode
        | Op::ReadFilled
        // A read is by a handle another word found: its table was reached there.
        | Op::ReadMade
        | Op::TestEqualConstant
        | Op::TestNotEqualConstant
        | Op::TestEqualRegister
        | Op::TestNotEqualRegister
        | Op::TestInConstants
        | Op::Let
        | Op::Yield
        | Op::Emit
        | Op::Axis => WordBuilt::Runs,
        Op::EachRowMade => {
            out.push(WalkPlace::Table(each_row_made_of(word).table));
            WordBuilt::Runs
        }
        Op::Latest => {
            if let Ok(site) = latest_site(code, latest_of(word).operands) {
                site_table(code, site, out);
            }
            WordBuilt::Runs
        }
        Op::Test => {
            let read = test_of(word);
            tested(comparison_of(read.comparison, read.regex))
        }
        Op::Exists => match body(code, exists_of(word).body, unit, out) {
            Some(_) => WordBuilt::Runs,
            None => WordBuilt::NotYet,
        },
        Op::Nearest => match body(code, nearest_of(word).operands, unit, out) {
            Some(_) => WordBuilt::Runs,
            None => WordBuilt::NotYet,
        },
        Op::Or => {
            let asking = or_of(word);
            let mut at = Some(asking.records);
            for _ in 0..asking.bodies.0 {
                at = at.and_then(|place| body(code, place, unit, out));
            }
            match at {
                Some(_) => WordBuilt::Runs,
                None => WordBuilt::NotYet,
            }
        }
        Op::Fork => {
            for entry in listed(&code.pool, fork_of(word).entries) {
                out.push(WalkPlace::Program {
                    entry: CodeEntry(entry.0),
                    unit,
                });
            }
            WordBuilt::Runs
        }
        // The loop does not run these yet.
        Op::EachRange
        | Op::Children
        | Op::ChildrenOfField
        | Op::Within
        | Op::NearestDerived
        | Op::Closure
        | Op::Call
        | Op::Render => WordBuilt::NotYet,
        // A stage is no word of a step program.
        Op::StageScan
        | Op::StageProbe
        | Op::StageLatest
        | Op::StageRead
        | Op::StageKeep
        | Op::StageCall
        | Op::StageProduct
        | Op::StageUse
        | Op::StageRender
        | Op::StageWalk
        | Op::StageWrite => WordBuilt::NotYet,
    }
}

/// The first unit `rule` can reach that holds a word or a fill the executor does not run; none when it runs them all.
pub(crate) fn unit_not_run(code: &ProgramCode, rule: &RuleCode) -> Option<WordSlot> {
    walked(code, rule, &mut Vec::new())
}

/// The units `rule` can reach, its own first: what the work ledger names the rule an owner of.
pub(crate) fn units_reached(code: &ProgramCode, rule: &RuleCode) -> Vec<WordSlot> {
    let mut units = Vec::new();
    let _unrun: Option<WordSlot> = walked(code, rule, &mut units);
    units
}

/// Walks all `rule` can reach, adding each unit met to `units` once; stops at the first unit that holds a word or a fill
/// the executor does not run, and answers it.
fn walked(code: &ProgramCode, rule: &RuleCode, units: &mut Vec<WordSlot>) -> Option<WordSlot> {
    let mut pending: Vec<WalkPlace> = Vec::new();
    let mut asked = Vec::new();
    for branch in &rule.branches {
        pending.push(WalkPlace::Program {
            entry: branch.entry,
            unit: rule.unit,
        });
        tables_asked(code, branch.need, &mut asked);
    }
    pending.extend(asked.into_iter().map(WalkPlace::Table));
    let (mut words, mut tables) = (
        vec![false; code.words.len()],
        vec![false; code.tables.len()],
    );
    while let Some(next) = pending.pop() {
        let (entry, unit) = match next {
            WalkPlace::Program { entry, unit } | WalkPlace::Pipeline { entry, unit } => {
                if !units.contains(&unit) {
                    units.push(unit);
                }
                (entry, unit)
            }
            WalkPlace::Table(table) => {
                let seen = tables.get_mut(usize::try_from(table.0).unwrap_or(usize::MAX));
                if let Some(seen) = seen.filter(|seen| !**seen) {
                    *seen = true;
                    let own = code
                        .tables
                        .get(usize::try_from(table.0).unwrap_or(usize::MAX));
                    if let Some(unit) = own
                        .map(|held| held.unit)
                        .filter(|unit| !units.contains(unit))
                    {
                        units.push(unit);
                    }
                    if let Some(unit) = fill(code, table, &mut pending) {
                        return Some(unit);
                    }
                }
                continue;
            }
        };
        let mut at = usize::try_from(entry.0).unwrap_or(usize::MAX);
        // A word walked before ends the walk: the words after it were walked then.
        while let Some(seen) = words.get_mut(at).filter(|seen| !**seen) {
            *seen = true;
            let Some(word) = code.words.get(at).copied() else {
                break;
            };
            let built = if matches!(next, WalkPlace::Pipeline { .. }) {
                stage_word(code, word, unit, &mut pending)
            } else {
                step_word(code, word, unit, &mut pending)
            };
            if built == WordBuilt::NotYet {
                return Some(unit);
            }
            // A step program ends at its `End`, a pipeline at its `StageWrite`.
            if word.op == Op::End || word.op == Op::StageWrite {
                break;
            }
            at = at.saturating_add(1);
        }
    }
    None
}
