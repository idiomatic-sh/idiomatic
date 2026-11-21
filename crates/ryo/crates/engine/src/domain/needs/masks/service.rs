//! The services of the `masks` part: the free terms and the clauses of a rule set's needs, found from the code once;
//! and whether a site's clauses leave it a way to hold in a file.

use super::types::{FreeTerm, NeedMasks, RecordCost, TermNumber};
use crate::domain::code::records::service::need_at;
use crate::domain::code::records::types::NeedRecord;
use crate::domain::code::types::{NeedSite, ProgramCode, WordSlot};
use ryo_relations::domain::catalog::service::node_extent;
use ryo_relations::domain::relations::types::Count;

/// The most clauses kept of one site, and the most an `Any` of several-clause parts is multiplied out to: a clause
/// left out is a thing not checked here, never a wrong answer.
const MOST_CLAUSES: Count = Count(24);

/// A clause while the masks are found: its terms, ascending.
type Clause = Vec<TermNumber>;

/// The slot `ahead` slots after a place among the needs.
fn slot(code: &ProgramCode, at: WordSlot, ahead: WordSlot) -> Option<WordSlot> {
    let place = at.0.checked_add(ahead.0)?;
    code.needs
        .get(usize::try_from(place).unwrap_or(usize::MAX))
        .copied()
}

/// The number of a free term, given the first time it is met.
fn numbered(masks: &mut NeedMasks, term: FreeTerm) -> TermNumber {
    let NeedMasks { terms, numbers, .. } = masks;
    *numbers.entry(term).or_insert_with(|| {
        terms.push(term);
        TermNumber(u32::try_from(terms.len().saturating_sub(1)).unwrap_or(u32::MAX))
    })
}

/// One clause of the terms of two.
fn joined(one: &Clause, other: &Clause) -> Clause {
    let mut all: Clause = one.iter().chain(other).copied().collect();
    all.sort_unstable();
    all.dedup();
    all
}

/// The clauses the record at a place cannot hold without.
fn record_clauses(code: &ProgramCode, at: WordSlot, masks: &mut NeedMasks) -> Vec<Clause> {
    let Some(record) = need_at(&code.needs, at) else {
        return Vec::new();
    };
    let parts_of = |count: WordSlot, offsets: WordSlot| {
        (0..count.0).filter_map(move |number| slot(code, offsets, WordSlot(number)))
    };
    match record {
        NeedRecord::Unknown | NeedRecord::Table(_) => Vec::new(),
        NeedRecord::Kinds(list) => vec![vec![numbered(masks, FreeTerm::Kinds(list))]],
        NeedRecord::Row {
            relation,
            column,
            constant,
        } => {
            let read = code
                .relations
                .get(usize::try_from(relation.0).unwrap_or(usize::MAX));
            if read.copied().and_then(node_extent).is_some() {
                let term = FreeTerm::NodeRow {
                    relation,
                    column,
                    constant,
                };
                vec![vec![numbered(masks, term)]]
            } else {
                Vec::new()
            }
        }
        NeedRecord::Site(site) => site_clauses(code, site, masks),
        NeedRecord::All {
            parts: count,
            offsets,
        } => {
            let mut all: Vec<Clause> = Vec::new();
            for place in parts_of(count, offsets) {
                for clause in record_clauses(code, place, masks) {
                    if all.len() < MOST_CLAUSES.0 && !all.contains(&clause) {
                        all.push(clause);
                    }
                }
            }
            all
        }
        NeedRecord::Any {
            parts: count,
            offsets,
        } => {
            // A part whose place cannot be read holds, by the walk: the whole says nothing here.
            let readable = parts_of(count, offsets).count();
            if u32::try_from(readable).unwrap_or(u32::MAX) != count.0 {
                return Vec::new();
            }
            let each: Vec<Vec<Clause>> = parts_of(count, offsets)
                .map(|place| record_clauses(code, place, masks))
                .collect();
            // A part that needs nothing here makes the whole need nothing here; no part at all never holds, which
            // the walk says.
            if each.is_empty() || each.iter().any(Vec::is_empty) {
                return Vec::new();
            }
            let product = each
                .iter()
                .fold(1_usize, |product, part| product.saturating_mul(part.len()));
            if product > MOST_CLAUSES.0 {
                // Some part holds, so some part's first clause does: one clause of them all.
                let first = each.iter().filter_map(|part| part.first());
                return vec![first.fold(Clause::new(), |all, clause| joined(&all, clause))];
            }
            // Every way to take one clause of each part: some part holds, so in each way some clause taken does.
            let mut ways: Vec<Clause> = vec![Clause::new()];
            for part in &each {
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
            ways
        }
    }
}

/// The clauses a site's formula cannot hold without, found once a site and kept. A site met while it is being found
/// (a formula that names itself) says nothing.
fn site_clauses(code: &ProgramCode, site: NeedSite, masks: &mut NeedMasks) -> Vec<Clause> {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    if let Some(Some(found)) = masks.of_site.get(place) {
        return found.clone();
    }
    if !masks.open.insert(site.0) {
        return Vec::new();
    }
    let clauses = match slot(code, WordSlot(site.0), WordSlot(0)) {
        Some(formula) => record_clauses(code, formula, masks),
        None => Vec::new(),
    };
    let _was = masks.open.remove(&site.0);
    if masks.of_site.len() <= place {
        masks.of_site.resize(place.saturating_add(1), None);
    }
    if let Some(held) = masks.of_site.get_mut(place) {
        *held = Some(clauses.clone());
    }
    clauses
}

/// Takes a site's clauses out of the masks, found now when this is the site's first decision: the caller asks their
/// terms of the file and gives them back with `clauses_back`.
pub(crate) fn clauses_taken(
    code: &ProgramCode,
    site: NeedSite,
    masks: &mut NeedMasks,
) -> Vec<Clause> {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    if !matches!(masks.of_site.get(place), Some(Some(_))) {
        let _found = site_clauses(code, site, masks);
    }
    match masks.of_site.get_mut(place) {
        Some(Some(held)) => std::mem::take(held),
        Some(None) | None => Vec::new(),
    }
}

/// Gives a site's clauses back to the masks.
pub(crate) fn clauses_back(site: NeedSite, clauses: Vec<Clause>, masks: &mut NeedMasks) {
    if let Some(held) = masks
        .of_site
        .get_mut(usize::try_from(site.0).unwrap_or(usize::MAX))
    {
        *held = Some(clauses);
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
        Some(NeedRecord::Row { relation, .. }) => {
            let read = code
                .relations
                .get(usize::try_from(relation.0).unwrap_or(usize::MAX));
            read.copied().and_then(node_extent).is_none()
        }
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
