//! The services of the `masks` part: the terms and the clauses of a rule set's needs, found from the code, each site at
//! its first decision; and what a record of the needs may cost, found once.

use super::types::{
    MaskExact, MustWord, NeedMasks, NeedTerm, NodeAsk, RecordCost, SiteMasks, SiteStretches,
    Stretch, TermNumber,
};
use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::records::service::need_at;
use crate::domain::code::records::types::NeedRecord;
use crate::domain::code::types::{NeedSite, ProgramCode, RelationNumber, WordSlot};
use crate::domain::steps::types::MadeTableId;
use ryo_relations::domain::catalog::service::node_extent;
use ryo_relations::domain::relations::types::Count;

/// The most clauses kept of one site or of one alternative, the most groups of one site and alternatives of one
/// group, and the most an `Any` of several-clause parts is multiplied out to (past it the `Any` is kept as a group):
/// what is left out past a bound makes the site's masks a part of its formula, never a wrong answer.
const MOST: (Count, Count, Count, Count) = (Count(64), Count(16), Count(64), Count(8));

/// A clause while the masks are found: its terms, ascending.
type Clause = Vec<TermNumber>;

/// What is found of a record or of a site: its clauses, its groups (each its alternatives, each its clauses), its
/// tables, and whether they are all it says.
#[derive(Clone, Debug)]
struct Found {
    clauses: Vec<Clause>,
    groups: Vec<Vec<Vec<Clause>>>,
    tables: Vec<MadeTableId>,
    exact: MaskExact,
}

/// What always holds, or of which nothing is kept.
const fn nothing(exact: MaskExact) -> Found {
    Found {
        clauses: Vec::new(),
        groups: Vec::new(),
        tables: Vec::new(),
        exact,
    }
}

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
pub(crate) fn numbered(
    code: &ProgramCode,
    masks: &mut NeedMasks,
    term: NeedTerm,
    at: WordSlot,
) -> TermNumber {
    let NeedMasks {
        terms,
        term_costs,
        numbers,
        leaves,
        free_places,
        free_terms,
        ..
    } = masks;
    let number = *numbers.entry(term).or_insert_with(|| {
        let number = TermNumber(u32::try_from(terms.len()).unwrap_or(u32::MAX));
        terms.push(term);
        let cost = match term {
            NeedTerm::Kinds(_) => RecordCost::Free,
            NeedTerm::Row { relation, .. } if node_relation(code, relation) => RecordCost::Free,
            NeedTerm::Row {
                column, constant, ..
            } if column == NO_SLOT || constant == NO_SLOT => RecordCost::Presence,
            NeedTerm::Row { .. } => RecordCost::Dear,
        };
        term_costs.push(cost);
        if cost == RecordCost::Free {
            free_terms.push(number);
            free_places.push(u32::try_from(free_terms.len()).unwrap_or(0));
        } else {
            free_places.push(0);
        }
        number
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

/// What the record at a place cannot hold without, and whether that is all it says.
fn record_found(code: &ProgramCode, at: WordSlot, masks: &mut NeedMasks) -> Found {
    // A record that cannot be read holds, by the walk.
    let Some(record) = need_at(&code.needs, at) else {
        return nothing(MaskExact::Whole);
    };
    match record {
        NeedRecord::Unknown => nothing(MaskExact::Whole),
        NeedRecord::Table(table) => Found {
            tables: vec![table],
            ..nothing(MaskExact::Whole)
        },
        NeedRecord::Kinds(list) => Found {
            clauses: vec![vec![numbered(code, masks, NeedTerm::Kinds(list), at)]],
            ..nothing(MaskExact::Whole)
        },
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
            Found {
                clauses: vec![vec![numbered(code, masks, term, at)]],
                ..nothing(MaskExact::Whole)
            }
        }
        NeedRecord::Site(site) => site_found(code, site, masks),
        NeedRecord::All {
            parts: count,
            offsets,
        } => {
            let mut all = nothing(MaskExact::Whole);
            for number in 0..count.0 {
                // A part whose place cannot be read is passed over by the walk.
                let Some(place) = slot(code, offsets, WordSlot(number)) else {
                    continue;
                };
                let part = record_found(code, place, masks);
                all.exact = both(all.exact, part.exact);
                for clause in part.clauses {
                    if all.clauses.contains(&clause) {
                        continue;
                    }
                    if all.clauses.len() < MOST.0.0 {
                        all.clauses.push(clause);
                    } else {
                        all.exact = MaskExact::Part;
                    }
                }
                for group in part.groups {
                    if all.groups.contains(&group) {
                        continue;
                    }
                    if all.groups.len() < MOST.1.0 {
                        all.groups.push(group);
                    } else {
                        all.exact = MaskExact::Part;
                    }
                }
                for table in part.tables {
                    if !all.tables.contains(&table) {
                        all.tables.push(table);
                    }
                }
            }
            all
        }
        NeedRecord::Any {
            parts: count,
            offsets,
        } => {
            let mut each: Vec<Found> = Vec::new();
            for number in 0..count.0 {
                // A part whose place cannot be read holds, by the walk: so does the whole.
                let Some(place) = slot(code, offsets, WordSlot(number)) else {
                    return nothing(MaskExact::Whole);
                };
                each.push(record_found(code, place, masks));
            }
            // No part at all never holds: one clause of no term.
            if each.is_empty() {
                return Found {
                    clauses: vec![Clause::new()],
                    ..nothing(MaskExact::Whole)
                };
            }
            // A part that asks a table is not kept as an alternative: its table is asked only when no other part
            // holds, which the walk knows. What is kept of it is then a part of what it says.
            let mut exact = MaskExact::Whole;
            for part in &mut each {
                if !part.tables.is_empty() {
                    part.tables.clear();
                    part.exact = MaskExact::Part;
                }
                exact = both(exact, part.exact);
            }
            // A part that always holds makes the whole always hold; a part nothing is kept of makes the whole unknown
            // here.
            let empty = |part: &Found| part.clauses.is_empty() && part.groups.is_empty();
            if each
                .iter()
                .any(|part| empty(part) && part.exact == MaskExact::Whole)
            {
                return nothing(MaskExact::Whole);
            }
            if each.iter().any(empty) {
                return nothing(MaskExact::Part);
            }
            let plain = each.iter().all(|part| part.groups.is_empty());
            let product = each.iter().fold(1_usize, |product, part| {
                product.saturating_mul(part.clauses.len().max(1))
            });
            if plain && product <= MOST.3.0 {
                // Every way to take one clause of each part: some part holds, so in each way some clause taken
                // does; and when in every way some clause holds, some part has all its clauses hold.
                let mut ways: Vec<Clause> = vec![Clause::new()];
                for part in &each {
                    let mut next =
                        Vec::with_capacity(ways.len().saturating_mul(part.clauses.len()));
                    for way in &ways {
                        for clause in &part.clauses {
                            next.push(joined(way, clause));
                        }
                    }
                    ways = next;
                }
                ways.sort_unstable();
                ways.dedup();
                return Found {
                    clauses: ways,
                    ..nothing(exact)
                };
            }
            // Kept as it is: one group, an alternative a part. A part that is itself one group and no more gives
            // its alternatives; any other part's own groups are left out of its alternative.
            let mut alternatives: Vec<Vec<Clause>> = Vec::new();
            for part in each {
                if part.clauses.is_empty() && part.groups.len() == 1 {
                    alternatives.extend(part.groups.into_iter().flatten());
                    continue;
                }
                if !part.groups.is_empty() || part.clauses.len() > MOST.0.0 {
                    exact = MaskExact::Part;
                }
                let kept = part.clauses.into_iter().take(MOST.0.0);
                alternatives.push(kept.collect());
            }
            alternatives.sort_unstable();
            alternatives.dedup();
            if alternatives.len() > MOST.2.0 {
                return nothing(MaskExact::Part);
            }
            // What every alternative needs, the group needs: those clauses are the site's own (most are of one free
            // term, decided by the and-compare), and the alternatives keep what tells them apart. An alternative
            // left with nothing holds whenever they do: the group is then no more than they.
            let common: Vec<Clause> = alternatives.first().map_or_else(Vec::new, |first| {
                let shared = first.iter().filter(|clause| {
                    alternatives
                        .iter()
                        .all(|alternative| alternative.contains(clause))
                });
                shared.cloned().collect()
            });
            for alternative in &mut alternatives {
                alternative.retain(|clause| !common.contains(clause));
            }
            let groups = if alternatives.iter().any(Vec::is_empty) {
                Vec::new()
            } else {
                vec![alternatives]
            };
            Found {
                clauses: common,
                groups,
                ..nothing(exact)
            }
        }
    }
}

/// The clauses at a stretch of the masks' clauses, each as its terms.
fn clauses_at(masks: &NeedMasks, stretch: Stretch) -> Vec<Clause> {
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
    each.collect()
}

/// What the masks hold of a site, when it was found.
fn held(masks: &NeedMasks, site: NeedSite) -> Option<Found> {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    let Some(SiteMasks::Found(at, exact)) = masks.sites.get(place).copied() else {
        return None;
    };
    let within =
        |stretch: Stretch| stretch.start.0..stretch.start.0.saturating_add(stretch.count.0);
    let groups = masks.groups.get(within(at.groups)).unwrap_or_default();
    let each = groups.iter().map(|group| {
        let alternatives = masks.alternatives.get(within(*group)).unwrap_or_default();
        alternatives
            .iter()
            .map(|alternative| clauses_at(masks, *alternative))
            .collect::<Vec<_>>()
    });
    Some(Found {
        clauses: clauses_at(masks, at.clauses),
        groups: each.collect(),
        tables: masks
            .tables
            .get(within(at.tables))
            .unwrap_or_default()
            .to_vec(),
        exact,
    })
}

/// Appends clauses to the masks, the clauses of terms that cost nothing first, the shorter first, and in a clause
/// those terms first; answers their stretch.
fn clauses_kept(masks: &mut NeedMasks, clauses: &mut [Clause]) -> Stretch {
    let cost = |term: &TermNumber| {
        let place = usize::try_from(term.0).unwrap_or(usize::MAX);
        masks.term_costs.get(place).copied().unwrap_or_default()
    };
    for clause in clauses.iter_mut() {
        clause.sort_by_key(|term| (cost(term), *term));
    }
    clauses.sort_by_key(|clause| (clause.iter().map(cost).max(), clause.len()));
    let start = Count(masks.clauses.len());
    for clause in clauses.iter() {
        masks.clauses.push(Stretch {
            start: Count(masks.clause_terms.len()),
            count: Count(clause.len()),
        });
        masks.clause_terms.extend_from_slice(clause);
    }
    Stretch {
        start,
        count: Count(clauses.len()),
    }
}

/// What a site's formula cannot hold without, found once a site and kept. A site met while it is being found (a
/// formula that names itself) says nothing of itself.
fn site_found(code: &ProgramCode, site: NeedSite, masks: &mut NeedMasks) -> Found {
    if let Some(found) = held(masks, site) {
        return found;
    }
    if !masks.open.insert(site.0) {
        return nothing(MaskExact::Part);
    }
    let mut found = match slot(code, WordSlot(site.0), WordSlot(0)) {
        Some(formula) => record_found(code, formula, masks),
        None => nothing(MaskExact::Whole),
    };
    let _was = masks.open.remove(&site.0);
    let clauses = clauses_kept(masks, &mut found.clauses);
    let mut groups = Vec::with_capacity(found.groups.len());
    for group in &mut found.groups {
        let mut alternatives = Vec::with_capacity(group.len());
        for alternative in group.iter_mut() {
            alternatives.push(clauses_kept(masks, alternative));
        }
        let start = Count(masks.alternatives.len());
        masks.alternatives.extend_from_slice(&alternatives);
        groups.push(Stretch {
            start,
            count: Count(alternatives.len()),
        });
    }
    let group_start = Count(masks.groups.len());
    masks.groups.extend_from_slice(&groups);
    let table_start = Count(masks.tables.len());
    masks.tables.extend_from_slice(&found.tables);
    // The clauses of one free term, as bits of the file's free answers, word by word.
    let mut words: Vec<MustWord> = Vec::new();
    for clause in &found.clauses {
        let [only] = clause.as_slice() else {
            continue;
        };
        let place = masks
            .free_places
            .get(usize::try_from(only.0).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or(0);
        let Some(index) = usize::try_from(place).unwrap_or(0).checked_sub(1) else {
            continue;
        };
        let (word, bits) = (Count(index / 64), 1_u64 << (index % 64));
        match words.iter_mut().find(|held| held.word == word) {
            Some(held) => held.bits |= bits,
            None => words.push(MustWord { word, bits }),
        }
    }
    let must_start = Count(masks.must.len());
    masks.must.extend_from_slice(&words);
    let at = SiteStretches {
        clauses,
        groups: Stretch {
            start: group_start,
            count: Count(groups.len()),
        },
        tables: Stretch {
            start: table_start,
            count: Count(found.tables.len()),
        },
        must: Stretch {
            start: must_start,
            count: Count(words.len()),
        },
    };
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    if masks.sites.len() <= place {
        masks
            .sites
            .resize(place.saturating_add(1), SiteMasks::Unfound);
    }
    if let Some(kept) = masks.sites.get_mut(place) {
        *kept = SiteMasks::Found(at, found.exact);
    }
    found
}

/// Where a site's clauses, groups and tables lie in the masks, and whether they are all of its formula: found now
/// when this is the site's first decision.
pub(crate) fn site_masks(
    code: &ProgramCode,
    site: NeedSite,
    masks: &mut NeedMasks,
) -> (SiteStretches, MaskExact) {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    if let Some(SiteMasks::Found(at, exact)) = masks.sites.get(place).copied() {
        return (at, exact);
    }
    let _found = site_found(code, site, masks);
    match masks.sites.get(place).copied() {
        Some(SiteMasks::Found(at, exact)) => (at, exact),
        Some(SiteMasks::Unfound) | None => (SiteStretches::default(), MaskExact::Part),
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

/// Whether the record at a place among the needs asks a relation of the node-table form for a row.
fn record_asks_nodes(code: &ProgramCode, at: WordSlot, masks: &mut NeedMasks) -> NodeAsk {
    match need_at(&code.needs, at) {
        Some(NeedRecord::Row { relation, .. }) if node_relation(code, relation) => NodeAsk::Asked,
        Some(NeedRecord::Site(site)) => node_asked(code, site, masks),
        Some(
            NeedRecord::All {
                parts: count,
                offsets,
            }
            | NeedRecord::Any {
                parts: count,
                offsets,
            },
        ) => {
            for number in 0..count.0 {
                if let Some(place) = slot(code, offsets, WordSlot(number))
                    && record_asks_nodes(code, place, masks) == NodeAsk::Asked
                {
                    return NodeAsk::Asked;
                }
            }
            NodeAsk::NotAsked
        }
        Some(
            NeedRecord::Unknown
            | NeedRecord::Kinds(_)
            | NeedRecord::Table(_)
            | NeedRecord::Row { .. },
        )
        | None => NodeAsk::NotAsked,
    }
}

/// Whether the formula of `site` asks a relation of the node-table form for a row, whatever the file: a rule that
/// consults such a decision read the node table. Found once a site and kept: a site many formulas name is walked once
/// (`needs::service::asks_nodes` walked it once for each formula that names it, at every level).
pub(crate) fn node_asked(code: &ProgramCode, site: NeedSite, masks: &mut NeedMasks) -> NodeAsk {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    match masks.node_asks.get(place).copied() {
        Some(NodeAsk::Asked) => return NodeAsk::Asked,
        Some(NodeAsk::NotAsked | NodeAsk::Finding) => return NodeAsk::NotAsked,
        Some(NodeAsk::Unknown) | None => {}
    }
    if masks.node_asks.len() <= place {
        masks
            .node_asks
            .resize(place.saturating_add(1), NodeAsk::Unknown);
    }
    if let Some(kept) = masks.node_asks.get_mut(place) {
        *kept = NodeAsk::Finding;
    }
    let found = match slot(code, WordSlot(site.0), WordSlot(0)) {
        Some(formula) => record_asks_nodes(code, formula, masks),
        None => NodeAsk::NotAsked,
    };
    if let Some(kept) = masks.node_asks.get_mut(place) {
        *kept = found;
    }
    found
}
