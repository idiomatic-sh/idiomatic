//! The services of the `budget` part: the run-time work budget, counted in rows. A unit keeps its own work and the list
//! of what it reached directly: a pattern's answer its program's rows and the units its program reached; a made table
//! its fill's rows (the fill side's slot) and what its fill read and asked. A rule is charged a unit the first time it
//! reaches it in the file, by a stamp, and follows the list, so each unit a rule reaches is charged to it once, exactly
//! as when the rule runs alone. One compare when a rule's work in a file ends; a rule over its allowance is noticed and
//! never stopped.

use super::types::FileSizes;
use super::types::UnitCaller;
use crate::domain::code::types::{RuleCode, WordSlot};
use crate::domain::executing::constants::{ALLOWANCE, ROWS_BESIDE, ROWS_PER_ELEMENT};
use crate::domain::findings::types::{HeaviestWork, InstructionCount, WorkNotice, WorkVerdict};
use crate::domain::made_tables::fills::service::fill_at;
use crate::domain::made_tables::fills::types::FillKind;
use crate::domain::made_tables::sources::service::{relation_rows, table_of};
use crate::domain::made_tables::types::{FillStore, Reached};
use crate::domain::memos::service::{charge_mut, reached_at, set_charge, summed};
use crate::domain::memos::types::{
    AnswersSum, ChargedUnit, EntryCharge, MemoId, MemoMap, RuleStamp,
};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::service::unit_step;
use crate::domain::programs::types::Programs;
use crate::domain::scratch::types::{FillTally, LoopState, RuleBudget, TableAsk, UnitTally};
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::node_table::service::element_count;
use ryo_language::domain::node_table::types::NodeTable;
use ryo_relations::domain::catalog::service::relation_size;
use ryo_relations::domain::relations::types::{Count, RelationSize};

/// Starts a pattern's evaluation: its own work is counted apart from its caller's.
pub(crate) fn begin_unit(state: &mut LoopState) -> UnitCaller {
    let caller = UnitCaller {
        visited: state.budget.visited,
        asked: Count(state.budget.reaching.len()),
    };
    state.budget.visited = InstructionCount(0);
    state.budget.depth = Count(state.budget.depth.0.saturating_add(1));
    caller
}

/// Ends a pattern's evaluation: the answer keeps its own work, its unit and the answers it asked directly, and the caller's
/// count goes on where it was.
pub(crate) fn end_unit(state: &mut LoopState, caller: UnitCaller, answer: MemoId, unit: WordSlot) {
    let LoopState { memos, budget, .. } = state;
    set_charge(
        memos,
        answer,
        EntryCharge {
            work: budget.visited,
            unit,
            ..EntryCharge::default()
        },
        budget.reaching.get(caller.asked.0..).unwrap_or_default(),
    );
    budget.reaching.truncate(caller.asked.0);
    budget.visited = caller.visited;
    budget.depth = Count(budget.depth.0.saturating_sub(1));
}

/// One unit's tally for the running rule, made nought when it was another rule's.
fn tally(budget: &mut RuleBudget, unit: WordSlot) -> Option<&mut UnitTally> {
    let place = usize::try_from(unit.0).unwrap_or(usize::MAX);
    if budget.tallies.len() <= place {
        budget
            .tallies
            .resize(place.saturating_add(1), UnitTally::default());
    }
    let stamp = budget.stamp;
    let kept = budget.tallies.get_mut(place)?;
    if kept.stamp != stamp {
        *kept = UnitTally {
            stamp,
            ..UnitTally::default()
        };
    }
    Some(kept)
}

/// Stamps the unit numbered `number` of `stamps` as charged to the running rule; answers the rule it was last charged
/// to when that was another, and nothing when the running rule was charged it before.
fn stamped(stamps: &mut Vec<RuleStamp>, number: Count, rule: RuleStamp) -> Option<RuleStamp> {
    if stamps.len() <= number.0 {
        stamps.resize(number.0.saturating_add(1), RuleStamp::default());
    }
    let last = stamps.get_mut(number.0).filter(|last| **last != rule)?;
    Some(std::mem::replace(last, rule))
}

/// How many rows a unit of the fill side holds now: what flows into a table that read it.
fn rows_of(store: &FillStore, nodes: &NodeTable, unit: Reached) -> Count {
    match unit {
        Reached::Table(table) => table_of(store, table).map_or(Count(0), |slot| slot.work.made),
        Reached::Relation(relation) => relation_rows(store, relation),
        Reached::Nodes => element_count(nodes),
        Reached::Index(_) => Count(0),
    }
}

/// Charges the running rule a made table: its fill's rows are work, the rows into it (what its fill read directly) and
/// out of it are terms of the allowance, and what its fill read and asked is charged next.
fn charge_table(
    budget: &mut RuleBudget,
    memos: &mut MemoMap,
    store: &FillStore,
    nodes: &NodeTable,
    table: MadeTableId,
) {
    let number = usize::try_from(table.0).unwrap_or(usize::MAX);
    if stamped(&mut budget.tables, Count(number), budget.stamp).is_none() {
        return;
    }
    let Some(slot) = table_of(store, table) else {
        return;
    };
    let rows = InstructionCount(
        u64::try_from(slot.work.read.0.saturating_add(slot.work.given.0)).unwrap_or(u64::MAX),
    );
    let into: usize = slot
        .reached
        .iter()
        .map(|each| rows_of(store, nodes, *each).0)
        .fold(0, usize::saturating_add);
    let terms = u64::try_from(into.saturating_add(slot.work.made.0)).unwrap_or(u64::MAX);
    budget.fills = InstructionCount(budget.fills.0.saturating_add(rows.0));
    budget.terms = InstructionCount(budget.terms.0.saturating_add(terms));
    if budget.heaviest.is_none_or(|kept| rows.0 > kept.rows.0) {
        budget.heaviest = Some(FillTally {
            table,
            rows,
            work: slot.work,
        });
    }
    budget
        .pending
        .extend(slot.reached.iter().map(|each| ChargedUnit::Fill(*each)));
    // SPIKE: the answers the table's fill asked are charged as one sum, made once a file for the table (and added to
    // when its fill asks more), not walked for every rule that reaches the table. The sum is the table's alone, so
    // the charge does not depend on which rule came first; an answer the rule also asked itself is counted in both.
    let sum = table_sum(budget, memos, number);
    budget.charged = InstructionCount(budget.charged.0.saturating_add(sum.work.0));
    budget.keys = InstructionCount(budget.keys.0.saturating_add(sum.keys.0));
    let RuleBudget {
        pending, sum_units, ..
    } = budget;
    let units = sum.units.start.0..sum.units.start.0.saturating_add(sum.units.length.0);
    pending.extend_from_slice(sum_units.get(units).unwrap_or_default());
}

/// SPIKE: the sum of the answers a table's fill asked in the file, brought up to its last ask.
fn table_sum(budget: &mut RuleBudget, memos: &mut MemoMap, number: usize) -> AnswersSum {
    let head = budget.asked.get(number).copied().unwrap_or_default();
    let known = budget.sums.get(number).copied().flatten();
    if let Some(sum) = known.filter(|sum| sum.upto == head) {
        return sum;
    }
    let RuleBudget {
        asks,
        sum_units,
        marks,
        ..
    } = &mut *budget;
    // The units the sum knew are copied to the list's end, where the new ones follow them.
    let start = sum_units.len();
    let mut sum = match known {
        Some(sum) => {
            let old = sum.units.start.0..sum.units.start.0.saturating_add(sum.units.length.0);
            sum_units.extend_from_within(old.start.min(start)..old.end.min(start));
            sum
        }
        None => {
            *marks = WordSlot(marks.0.wrapping_add(1));
            AnswersSum {
                mark: *marks,
                ..AnswersSum::default()
            }
        }
    };
    let stop = sum.upto;
    sum.units.start = Count(start);
    let mut place = head;
    let answers = std::iter::from_fn(|| {
        if place == stop {
            return None;
        }
        let ask = place.0.checked_sub(1).and_then(|at| asks.get(at))?;
        place = ask.before;
        Some(ask.answer)
    });
    let mut sum = summed(memos, answers, sum, sum_units);
    sum.upto = head;
    if budget.sums.len() <= number && number < usize::MAX {
        budget.sums.resize(number.saturating_add(1), None);
    }
    if let Some(slot) = budget.sums.get_mut(number) {
        *slot = Some(sum);
    }
    sum
}

/// Charges the running rule a unit and everything the unit's lists reach, each once.
fn charge(frame: &mut StepFrame<'_, '_>, unit: ChargedUnit) {
    let (store, nodes) = (&*frame.fill.store, frame.fill.nodes);
    let LoopState { memos, budget, .. } = &mut *frame.state;
    budget.pending.push(unit);
    while let Some(next) = budget.pending.pop() {
        match next {
            ChargedUnit::Answer(answer) => {
                let Some(entry) = charge_mut(memos, answer) else {
                    continue;
                };
                if entry.charged == budget.stamp {
                    continue;
                }
                entry.charged = budget.stamp;
                let (work, unit, asked) = (entry.work, entry.unit, entry.reached);
                budget.charged = InstructionCount(budget.charged.0.saturating_add(work.0));
                budget.keys = InstructionCount(budget.keys.0.saturating_add(1));
                if let Some(kept) = tally(budget, unit) {
                    kept.keys = InstructionCount(kept.keys.0.saturating_add(1));
                    kept.visited = InstructionCount(kept.visited.0.saturating_add(work.0));
                }
                budget.pending.extend_from_slice(reached_at(memos, asked));
            }
            ChargedUnit::Fill(Reached::Table(table)) => {
                charge_table(budget, memos, store, nodes, table);
            }
            ChargedUnit::Fill(Reached::Relation(relation)) => {
                let number = usize::try_from(relation.0).unwrap_or(usize::MAX);
                if stamped(&mut budget.relations, Count(number), budget.stamp).is_some() {
                    budget.read.push(relation);
                }
            }
            ChargedUnit::Fill(Reached::Nodes) => {
                if budget.nodes != budget.stamp {
                    budget.nodes = budget.stamp;
                    let elements = u64::try_from(element_count(nodes).0).unwrap_or(u64::MAX);
                    budget.terms = InstructionCount(budget.terms.0.saturating_add(elements));
                }
            }
            // An index is no rows of its own: its source is on the list beside it.
            ChargedUnit::Fill(Reached::Index(_)) => {}
        }
    }
}

/// Whether the running rule was charged a unit before: one compare, with no list walked.
fn charged_before(budget: &RuleBudget, memos: &MemoMap, unit: ChargedUnit) -> Option<RuleStamp> {
    let stamp = match unit {
        ChargedUnit::Answer(answer) => memos
            .charges
            .get(usize::try_from(answer.0).unwrap_or(usize::MAX))
            .map(|entry| entry.charged),
        ChargedUnit::Fill(Reached::Table(table)) => budget
            .tables
            .get(usize::try_from(table.0).unwrap_or(usize::MAX))
            .copied(),
        ChargedUnit::Fill(Reached::Relation(relation)) => budget
            .relations
            .get(usize::try_from(relation.0).unwrap_or(usize::MAX))
            .copied(),
        ChargedUnit::Fill(Reached::Nodes) => Some(budget.nodes),
        // An index is never charged: it is always as good as charged.
        ChargedUnit::Fill(Reached::Index(_)) => Some(budget.stamp),
    };
    stamp.filter(|charged| *charged == budget.stamp)
}

/// A unit was reached: a pattern's answer asked, kept or just evaluated; a table, a relation or the node table read.
/// Reached by the rule's own program, the rule is charged it; reached inside a pattern's evaluation, it goes on that
/// pattern's list, and the rule is charged it when it reaches the pattern. This is called for every read of a source,
/// so a unit the rule was charged before costs one compare, and a unit reached again and again by the same program is
/// on the pattern's list once for each run of reaches, not once for each.
pub(crate) fn reach(frame: &mut StepFrame<'_, '_>, unit: ChargedUnit) {
    let LoopState { budget, memos, .. } = &mut *frame.state;
    if budget.depth.0 == 0 {
        if charged_before(budget, memos, unit).is_none() {
            charge(frame, unit);
        }
    } else if budget.reaching.last() != Some(&unit) {
        budget.reaching.push(unit);
    }
}

/// A table's fill asked a pattern for an answer: the answer goes on the table's list, which the fill side does not
/// keep, so a rule that reaches the table later is charged the answer as the rule that made the table was.
pub(crate) fn fill_asked(state: &mut LoopState, table: MadeTableId, answer: MemoId) {
    let budget = &mut state.budget;
    let number = usize::try_from(table.0).unwrap_or(usize::MAX);
    if budget.asked.len() <= number {
        budget.asked.resize(number.saturating_add(1), Count(0));
    }
    if let Some(last) = budget.asked.get_mut(number) {
        budget.asks.push(TableAsk {
            answer,
            before: *last,
        });
        *last = Count(budget.asks.len());
    }
}

/// SPIKE: a table's fill asked a pattern whose answer was not kept: its work and its key go to the table's sum.
pub(crate) fn fill_passed(state: &mut LoopState, table: MadeTableId, work: InstructionCount) {
    let budget = &mut state.budget;
    let number = usize::try_from(table.0).unwrap_or(usize::MAX);
    if budget.sums.len() <= number && number < usize::MAX {
        budget.sums.resize(number.saturating_add(1), None);
    }
    let Some(slot) = budget.sums.get_mut(number) else {
        return;
    };
    let mut sum = match *slot {
        Some(sum) => sum,
        None => {
            budget.marks = WordSlot(budget.marks.0.wrapping_add(1));
            AnswersSum {
                mark: budget.marks,
                ..AnswersSum::default()
            }
        }
    };
    sum.work = InstructionCount(sum.work.0.saturating_add(work.0));
    sum.keys = InstructionCount(sum.keys.0.saturating_add(1));
    *slot = Some(sum);
}

/// Starts a rule's work in a file: nothing is charged to it yet.
pub(crate) fn begin_rule(state: &mut LoopState) {
    let budget = &mut state.budget;
    budget.stamp = RuleStamp(budget.stamp.0.wrapping_add(1));
    budget.visited = InstructionCount(0);
    budget.candidates = InstructionCount(0);
    budget.charged = InstructionCount(0);
    budget.keys = InstructionCount(0);
    budget.fills = InstructionCount(0);
    budget.terms = InstructionCount(0);
    budget.heaviest = None;
    budget.depth = Count(0);
    budget.reaching.clear();
    budget.read.clear();
}

/// What the notice says of a fill: its table's kind of work, from the slot's counts.
fn fill_work(programs: &Programs, fill: FillTally) -> HeaviestWork {
    let counted = |rows: Count| InstructionCount(u64::try_from(rows.0).unwrap_or(u64::MAX));
    let (read, made) = (counted(fill.work.read), counted(fill.work.made));
    let kind = fill_at(&programs.0.code, fill.table).map(|held| held.kind);
    // An ordered match's two counts are its two inputs' rows; any other fill's are its product stage's sides.
    if matches!(kind, Ok(FillKind::Match)) {
        return HeaviestWork::OrderedPairs {
            left: counted(fill.work.left),
            right: counted(fill.work.right),
            made,
        };
    }
    if fill.work.left.0 != 0 {
        return HeaviestWork::Product {
            left: counted(fill.work.left),
            right: counted(fill.work.right),
        };
    }
    match kind {
        Ok(FillKind::Count | FillKind::Ends | FillKind::Places) => {
            HeaviestWork::Group { read, made }
        }
        Ok(FillKind::Sorted) => HeaviestWork::Sorted { rows: made },
        Ok(FillKind::Follow | FillKind::Visits) => HeaviestWork::Graph { read, made },
        Ok(FillKind::Rows | FillKind::Constant | FillKind::Match) | Err(_) => {
            HeaviestWork::Rows { read, made }
        }
    }
}

/// A relation that wrote more rows than its declared size allows is said too (`Oversized`), with the same counts: a
/// relation of one row a file past one row, a relation that grows with the file's elements past `ROWS_PER_ELEMENT` rows an
/// element and `ROWS_BESIDE`. Neither verdict stops the rule.
///
/// Ends a rule's work in a file: the one compare. Its work is the rows its own program's iterations gave, the work of
/// every answer charged to it, and the rows the fills charged to it read and gave. Its allowance is `ALLOWANCE` times
/// its candidates, the keys it asked of call patterns, the rows of each source it read once (the node table's elements,
/// each relation's rows as the file holds them now) and the rows into and out of each table it reached. Past it, the
/// notice names the unit that did the most counted work: a fill before a pattern before the rule's own program on a tie,
/// and among patterns the first in unit order.
pub(crate) fn end_rule(
    state: &LoopState,
    sizes: FileSizes<'_>,
    programs: &Programs,
    rule: &RuleCode,
) -> Vec<WorkNotice> {
    let budget = &state.budget;
    let store = sizes.store;
    let work = budget
        .visited
        .0
        .saturating_add(budget.charged.0)
        .saturating_add(budget.fills.0);
    let sources = budget
        .read
        .iter()
        .map(|relation| u64::try_from(relation_rows(store, *relation).0).unwrap_or(u64::MAX))
        .fold(0, u64::saturating_add);
    let allowance = ALLOWANCE.0.saturating_mul(
        budget
            .candidates
            .0
            .saturating_add(budget.keys.0)
            .saturating_add(budget.terms.0)
            .saturating_add(sources),
    );
    let code = &programs.0.code;
    let elements = element_count(sizes.nodes).0;
    let oversized = budget.read.iter().filter_map(|number| {
        let relation = code
            .relations
            .get(usize::try_from(number.0).unwrap_or(usize::MAX))
            .copied()?;
        let bound = match relation_size(relation)? {
            RelationSize::PerFile => 1,
            RelationSize::PerElement => elements
                .saturating_mul(ROWS_PER_ELEMENT.0)
                .saturating_add(ROWS_BESIDE.0),
            RelationSize::PerPair => usize::MAX,
        };
        (relation_rows(store, *number).0 > bound).then_some(WorkVerdict::Oversized(relation))
    });
    let over = (work > allowance).then_some(WorkVerdict::Over);
    let verdicts: Vec<WorkVerdict> = over.into_iter().chain(oversized).collect();
    if verdicts.is_empty() {
        return Vec::new();
    }
    let mut heaviest = (
        rule.unit,
        budget.visited,
        HeaviestWork::Program {
            candidates: budget.candidates,
            visited: budget.visited,
        },
    );
    for (place, kept) in budget.tallies.iter().enumerate() {
        let unit = WordSlot(u32::try_from(place).unwrap_or(u32::MAX));
        let heavier = kept.visited.0 > heaviest.1.0
            || (kept.visited.0 == heaviest.1.0 && unit.0 < heaviest.0.0);
        if kept.stamp == budget.stamp && heavier {
            heaviest = (
                unit,
                kept.visited,
                HeaviestWork::Calls {
                    keys: kept.keys,
                    visited: kept.visited,
                },
            );
        }
    }
    if let Some(fill) = budget.heaviest.filter(|fill| fill.rows.0 >= heaviest.1.0)
        && let Some(table) = programs
            .0
            .code
            .tables
            .get(usize::try_from(fill.table.0).unwrap_or(usize::MAX))
    {
        heaviest = (table.unit, fill.rows, fill_work(programs, fill));
    }
    let Some(step) = code
        .units
        .get(usize::try_from(heaviest.0.0).unwrap_or(usize::MAX))
        .map(|unit| unit_step(programs, rule.rule, &unit.key))
    else {
        return Vec::new();
    };
    let said = |verdict| WorkNotice {
        rule: rule.rule,
        verdict,
        work: InstructionCount(work),
        allowance: InstructionCount(allowance),
        step,
        heaviest: heaviest.2,
    };
    verdicts.into_iter().map(said).collect()
}
