//! The services of the `masks` part: the terms and the clauses of a rule set's needs, found from the code, each site at
//! its first decision; and what a record of the needs may cost, found once.

use super::types::{MaskExact, NeedMasks, NeedTerm, RecordCost, SiteMasks, Stretch, TermNumber};
use crate::domain::code::records::service::need_at;
use crate::domain::code::records::types::NeedRecord;
use crate::domain::code::types::{NeedSite, ProgramCode, RelationNumber, WordSlot};
use ryo_relations::domain::catalog::service::node_extent;
use ryo_relations::domain::relations::types::Count;

/// The most clauses kept of one site, and the most an `Any` of several-clause parts is multiplied out to: a clause
/// left out makes the site's clauses a part of its formula, never a wrong answer.
const MOST_CLAUSES: Count = Count(32);

/// A clause while the masks are found: its terms, ascending.
type Clause = Vec<TermNumber>;

/// The slot `ahead` slots after a place among the needs.
fn slot(code: &ProgramCode, at: WordSlot, ahead: WordSlot) -> Option<WordSlot> {
    let place = at.0.checked_add(ahead.0)?;
    code.needs
        .get(usize::try_from(place).unwrap_or(usize::MAX))
        .copied()
}

/// Whether a relation is of the node-table form: a row of it costs nothing to ask.
fn node_relation(code: &ProgramCode, relation: RelationNumber) -> bool {
    let read = code
        .relations
        .get(usize::try_from(relation.0).unwrap_or(usize::MAX));
    read.copied().and_then(node_extent).is_some()
}

/// The number of a term, given the first time it is met; the record at `at` is that term.
fn numbered(code: &ProgramCode, masks: &mut NeedMasks, term: NeedTerm, at: WordSlot) -> TermNumber {
    let NeedMasks {
        terms,
        term_costs,
        numbers,
        leaves,
        ..
    } = masks;
    let number = *numbers.entry(term).or_insert_with(|| {
        terms.push(term);
        term_costs.push(match term {
            NeedTerm::Kinds(_) => RecordCost::Free,
            NeedTerm::Row { relation, .. } if node_relation(code, relation) => RecordCost::Free,
            NeedTerm::Row { .. } => RecordCost::Dear,
        });
        TermNumber(u32::try_from(terms.len().saturating_sub(1)).unwrap_or(u32::MAX))
    });
    let place = usize::try_from(at.0).unwrap_or(usize::MAX);
    if leaves.len() <= place {
        leaves.resize(place.saturating_add(1), 0);
    }
    if let Some(leaf) = leaves.get_mut(place) {
        *leaf = number.0.saturating_add(1);
    }
    number
}

/// One clause of the terms of two.
fn joined(one: &Clause, other: &Clause) -> Clause {
    let mut all: Clause = one.iter().chain(other).copied().collect();
    all.sort_unstable();
    all.dedup();
    all
}

/// Both exact, or a part.
const fn both(one: MaskExact, other: MaskExact) -> MaskExact {
    match (one, other) {
        (MaskExact::Whole, MaskExact::Whole) => MaskExact::Whole,
        (MaskExact::Part, MaskExact::Whole | MaskExact::Part)
        | (MaskExact::Whole, MaskExact::Part) => MaskExact::Part,
    }
}

/// The clauses the record at a place cannot hold without, and whether they are all it says.
fn record_clauses(
    code: &ProgramCode,
    at: WordSlot,
    masks: &mut NeedMasks,
) -> (Vec<Clause>, MaskExact) {
    // A record that cannot be read holds, by the walk.
    let Some(record) = need_at(&code.needs, at) else {
        return (Vec::new(), MaskExact::Whole);
    };
    match record {
        NeedRecord::Unknown => (Vec::new(), MaskExact::Whole),
        NeedRecord::Table(_) => (Vec::new(), MaskExact::Part),
        NeedRecord::Kinds(list) => (
            vec![vec![numbered(code, masks, NeedTerm::Kinds(list), at)]],
            MaskExact::Whole,
        ),
        NeedRecord::Row {
            relation,
            column,
            constant,
        } => {
            let term = NeedTerm::Row {
                relation,
                column,
                constant,
            };
            (
                vec![vec![numbered(code, masks, term, at)]],
                MaskExact::Whole,
            )
        }
        NeedRecord::Site(site) => site_clauses(code, site, masks),
        NeedRecord::All {
            parts: count,
            offsets,
        } => {
            let (mut all, mut exact): (Vec<Clause>, MaskExact) = (Vec::new(), MaskExact::Whole);
            for number in 0..count.0 {
                // A part whose place cannot be read is passed over by the walk.
                let Some(place) = slot(code, offsets, WordSlot(number)) else {
                    continue;
                };
                let (clauses, part) = record_clauses(code, place, masks);
                exact = both(exact, part);
                for clause in clauses {
                    if all.contains(&clause) {
                        continue;
                    }
                    if all.len() < MOST_CLAUSES.0 {
                        all.push(clause);
                    } else {
                        exact = MaskExact::Part;
                    }
                }
            }
            (all, exact)
        }
        NeedRecord::Any {
            parts: count,
            offsets,
        } => {
            let mut each: Vec<(Vec<Clause>, MaskExact)> = Vec::new();
            for number in 0..count.0 {
                // A part whose place cannot be read holds, by the walk: so does the whole.
                let Some(place) = slot(code, offsets, WordSlot(number)) else {
                    return (Vec::new(), MaskExact::Whole);
                };
                each.push(record_clauses(code, place, masks));
            }
            // No part at all never holds: one clause of no term.
            if each.is_empty() {
                return (vec![Clause::new()], MaskExact::Whole);
            }
            // A part that always holds makes the whole always hold; one of which nothing is known here makes the
            // whole unknown here.
            if each
                .iter()
                .any(|(clauses, exact)| clauses.is_empty() && *exact == MaskExact::Whole)
            {
                return (Vec::new(), MaskExact::Whole);
            }
            if each.iter().any(|(clauses, _)| clauses.is_empty()) {
                return (Vec::new(), MaskExact::Part);
            }
            let exact = each
                .iter()
                .fold(MaskExact::Whole, |exact, (_, part)| both(exact, *part));
            let product = each.iter().fold(1_usize, |product, (part, _)| {
                product.saturating_mul(part.len())
            });
            if product > MOST_CLAUSES.0 {
                // Some part holds, so some part's first clause does: one clause of them all.
                let first = each.iter().filter_map(|(part, _)| part.first());
                let one = first.fold(Clause::new(), |all, clause| joined(&all, clause));
                return (vec![one], MaskExact::Part);
            }
            // Every way to take one clause of each part: some part holds, so in each way some clause taken does; and
            // when in every way some clause holds, some part has all its clauses hold.
            let mut ways: Vec<Clause> = vec![Clause::new()];
            for (part, _) in &each {
                let mut next = Vec::with_capacity(ways.len().saturating_mul(part.len()));
                for way in &ways {
                    for clause in part {
                        next.push(joined(way, clause));
                    }
                }
                ways = next;
            }
            ways.sort_unstable();
            ways.dedup();
            (ways, exact)
        }
    }
}

/// A site's clauses as the masks hold them, when it was found.
fn held(masks: &NeedMasks, site: NeedSite) -> Option<(Vec<Clause>, MaskExact)> {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    let Some(SiteMasks::Found(stretch, exact)) = masks.sites.get(place).copied() else {
        return None;
    };
    let clauses = masks
        .clauses
        .get(stretch.start.0..stretch.start.0.saturating_add(stretch.count.0))
        .unwrap_or_default();
    let each = clauses.iter().map(|clause| {
        masks
            .clause_terms
            .get(clause.start.0..clause.start.0.saturating_add(clause.count.0))
            .unwrap_or_default()
            .to_vec()
    });
    Some((each.collect(), exact))
}

/// The clauses a site's formula cannot hold without, found once a site and kept: the clauses of terms that cost
/// nothing first, the shorter first, and in a clause those terms first. A site met while it is being found (a formula
/// that names itself) says nothing of itself.
fn site_clauses(
    code: &ProgramCode,
    site: NeedSite,
    masks: &mut NeedMasks,
) -> (Vec<Clause>, MaskExact) {
    if let Some(found) = held(masks, site) {
        return found;
    }
    if !masks.open.insert(site.0) {
        return (Vec::new(), MaskExact::Part);
    }
    let (mut clauses, exact) = match slot(code, WordSlot(site.0), WordSlot(0)) {
        Some(formula) => record_clauses(code, formula, masks),
        None => (Vec::new(), MaskExact::Whole),
    };
    let _was = masks.open.remove(&site.0);
    let cost = |term: &TermNumber| {
        let place = usize::try_from(term.0).unwrap_or(usize::MAX);
        masks.term_costs.get(place).copied().unwrap_or_default()
    };
    for clause in &mut clauses {
        clause.sort_by_key(|term| (cost(term), *term));
    }
    clauses.sort_by_key(|clause| (clause.iter().map(cost).max(), clause.len()));
    let start = Count(masks.clauses.len());
    for clause in &clauses {
        masks.clauses.push(Stretch {
            start: Count(masks.clause_terms.len()),
            count: Count(clause.len()),
        });
        masks.clause_terms.extend_from_slice(clause);
    }
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    if masks.sites.len() <= place {
        masks
            .sites
            .resize(place.saturating_add(1), SiteMasks::Unfound);
    }
    let stretch = Stretch {
        start,
        count: Count(clauses.len()),
    };
    if let Some(held) = masks.sites.get_mut(place) {
        *held = SiteMasks::Found(stretch, exact);
    }
    (clauses, exact)
}

/// A site's clauses, a stretch of the masks' clauses, and whether they are all of its formula: found now when this
/// is the site's first decision.
pub(crate) fn site_masks(
    code: &ProgramCode,
    site: NeedSite,
    masks: &mut NeedMasks,
) -> (Stretch, MaskExact) {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    if let Some(SiteMasks::Found(stretch, exact)) = masks.sites.get(place).copied() {
        return (stretch, exact);
    }
    let _found = site_clauses(code, site, masks);
    match masks.sites.get(place).copied() {
        Some(SiteMasks::Found(stretch, exact)) => (stretch, exact),
        Some(SiteMasks::Unfound) | None => (Stretch::default(), MaskExact::Part),
    }
}

/// Whether the record at a place may cost a fill, whatever the file: found once a place.
fn record_cost(code: &ProgramCode, at: WordSlot, costs: &mut Vec<RecordCost>) -> RecordCost {
    let place = usize::try_from(at.0).unwrap_or(usize::MAX);
    match costs.get(place) {
        Some(RecordCost::Unknown) => {}
        Some(known) => return *known,
        None => return RecordCost::Dear,
    }
    // A formula that names itself is taken as dear while it is found.
    if let Some(held) = costs.get_mut(place) {
        *held = RecordCost::Dear;
    }
    let dear = match need_at(&code.needs, at) {
        Some(NeedRecord::Unknown | NeedRecord::Kinds(_)) | None => false,
        Some(NeedRecord::Row { relation, .. }) => !node_relation(code, relation),
        Some(NeedRecord::Site(site)) => slot(code, WordSlot(site.0), WordSlot(0))
            .is_some_and(|formula| record_cost(code, formula, costs) == RecordCost::Dear),
        Some(NeedRecord::Table(_)) => true,
        Some(
            NeedRecord::All {
                parts: count,
                offsets,
            }
            | NeedRecord::Any {
                parts: count,
                offsets,
            },
        ) => (0..count.0).any(|number| {
            slot(code, offsets, WordSlot(number))
                .is_some_and(|part| record_cost(code, part, costs) == RecordCost::Dear)
        }),
    };
    let cost = if dear {
        RecordCost::Dear
    } else {
        RecordCost::Free
    };
    if let Some(held) = costs.get_mut(place) {
        *held = cost;
    }
    cost
}

/// Finds, for every place among the needs, whether its record may cost a fill: once for a worker.
pub(crate) fn costs_found(code: &ProgramCode, masks: &mut NeedMasks) {
    masks.costs.clear();
    masks.costs.resize(code.needs.len(), RecordCost::Unknown);
    for place in 0..code.needs.len() {
        // Only a place that starts a record is asked by the walk; any other is found dear or free and never read.
        let at = WordSlot(u32::try_from(place).unwrap_or(u32::MAX));
        let _cost = record_cost(code, at, &mut masks.costs);
    }
}
