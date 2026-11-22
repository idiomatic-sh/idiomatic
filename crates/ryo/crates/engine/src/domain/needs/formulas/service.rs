//! The services of the `formulas` part: a site's formula compiled from the code's records into nodes, once a worker at
//! the site's first decision. A record met again (a part many formulas share) is compiled once; another site is named,
//! never copied in.

use super::types::{AskRank, FormulaNode};
use crate::domain::code::records::service::need_at;
use crate::domain::code::records::types::NeedRecord;
use crate::domain::code::types::{NeedSite, ProgramCode, WordSlot};
use crate::domain::needs::masks::service::numbered;
use crate::domain::needs::masks::types::{MustWord, NeedMasks, NeedTerm, RecordCost, Stretch};
use ryo_relations::domain::relations::types::Count;

/// A root being compiled.
const COMPILING: u32 = u32::MAX;

/// The slot `ahead` slots after a place among the needs.
fn slot(code: &ProgramCode, at: WordSlot, ahead: WordSlot) -> Option<WordSlot> {
    let place = at.0.checked_add(ahead.0)?;
    code.needs
        .get(usize::try_from(place).unwrap_or(usize::MAX))
        .copied()
}

/// A node added, with what asking it may cost; answers its number.
fn pushed(masks: &mut NeedMasks, node: FormulaNode, rank: AskRank) -> u32 {
    let formulas = &mut masks.formulas;
    formulas.nodes.push(node);
    formulas.ranks.push(rank);
    u32::try_from(formulas.nodes.len().saturating_sub(1)).unwrap_or(u32::MAX)
}

/// What asking a node may cost.
pub(crate) fn rank_of(masks: &NeedMasks, node: u32) -> AskRank {
    masks
        .formulas
        .ranks
        .get(usize::try_from(node).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or(AskRank::Presence)
}

/// The node of a term: a bit of the file's free answers when it costs nothing, else the term by number.
fn term_node(code: &ProgramCode, masks: &mut NeedMasks, term: NeedTerm, at: WordSlot) -> u32 {
    let number = numbered(code, masks, term, at);
    let place = usize::try_from(number.0).unwrap_or(usize::MAX);
    let cost = masks.term_costs.get(place).copied().unwrap_or_default();
    let free = masks.free_places.get(place).copied().unwrap_or(0);
    match (cost, free.checked_sub(1)) {
        (RecordCost::Free, Some(bit)) => pushed(masks, FormulaNode::Free(bit), AskRank::Free),
        (RecordCost::Presence, _) => pushed(masks, FormulaNode::Term(number), AskRank::Presence),
        (RecordCost::Unknown | RecordCost::Free | RecordCost::Dear, _) => {
            pushed(masks, FormulaNode::Term(number), AskRank::Row)
        }
    }
}

/// The node of an `All` or an `Any` of the parts at `offsets`: its parts compiled and ordered by what asking them may cost
/// (the record's order among parts of one rank). A part of an `All` that cannot be read is passed over; one of an `Any`
/// holds.
fn parts_node(
    code: &ProgramCode,
    masks: &mut NeedMasks,
    count: WordSlot,
    offsets: WordSlot,
    every: bool,
) -> u32 {
    let mut parts: Vec<u32> = Vec::with_capacity(usize::try_from(count.0).unwrap_or(0));
    for number in 0..count.0 {
        match slot(code, offsets, WordSlot(number)) {
            Some(place) => parts.push(compiled(code, place, masks)),
            None if every => {}
            None => parts.push(pushed(masks, FormulaNode::Always, AskRank::Free)),
        }
    }
    // The parts that are free terms become one node of bit words: a word's and-compare in place of a node a term.
    let mut words: Vec<MustWord> = Vec::new();
    parts.retain(|part| {
        let Some(FormulaNode::Free(bit)) = masks
            .formulas
            .nodes
            .get(usize::try_from(*part).unwrap_or(usize::MAX))
            .copied()
        else {
            return true;
        };
        let place = usize::try_from(bit).unwrap_or(usize::MAX);
        let (word, held) = (Count(place / 64), 1_u64 << (place % 64));
        match words.iter_mut().find(|each| each.word == word) {
            Some(each) => each.bits |= held,
            None => words.push(MustWord { word, bits: held }),
        }
        false
    });
    if words.len() == 1 && parts.is_empty() && words.iter().all(|each| each.bits.count_ones() == 1) {
        // One free term alone is its own bit.
        let bit = words
            .first()
            .map_or(0, |each| each.word.0 * 64 + usize::try_from(each.bits.trailing_zeros()).unwrap_or(0));
        return pushed(masks, FormulaNode::Free(u32::try_from(bit).unwrap_or(u32::MAX)), AskRank::Free);
    }
    if !words.is_empty() {
        let stretch = Stretch {
            start: Count(masks.formulas.bits.len()),
            count: Count(words.len()),
        };
        masks.formulas.bits.extend_from_slice(&words);
        let node = if every {
            FormulaNode::AllBits(stretch)
        } else {
            FormulaNode::AnyBits(stretch)
        };
        let bits = pushed(masks, node, AskRank::Free);
        if parts.is_empty() {
            return bits;
        }
        parts.insert(0, bits);
    }
    if parts.is_empty() {
        let node = if every {
            FormulaNode::Always
        } else {
            FormulaNode::Never
        };
        return pushed(masks, node, AskRank::Free);
    }
    parts.sort_by_key(|part| rank_of(masks, *part));
    let rank = parts
        .iter()
        .map(|part| rank_of(masks, *part))
        .max()
        .unwrap_or_default();
    let stretch = Stretch {
        start: Count(masks.formulas.parts.len()),
        count: Count(parts.len()),
    };
    masks.formulas.parts.extend_from_slice(&parts);
    let node = if every {
        FormulaNode::All(stretch)
    } else {
        FormulaNode::Any(stretch)
    };
    pushed(masks, node, rank)
}

/// The node compiled from the record at a place among the needs: compiled once, read again after.
fn compiled(code: &ProgramCode, at: WordSlot, masks: &mut NeedMasks) -> u32 {
    let place = usize::try_from(at.0).unwrap_or(usize::MAX);
    if let Some(node) = masks
        .formulas
        .at_place
        .get(place)
        .copied()
        .and_then(|held| held.checked_sub(1))
    {
        return node;
    }
    let node = match need_at(&code.needs, at) {
        None | Some(NeedRecord::Unknown) => pushed(masks, FormulaNode::Always, AskRank::Free),
        Some(NeedRecord::Kinds(list)) => term_node(code, masks, NeedTerm::Kinds(list), at),
        Some(NeedRecord::Row {
            relation,
            column,
            constant,
        }) => term_node(
            code,
            masks,
            NeedTerm::Row {
                relation,
                column,
                constant,
            },
            at,
        ),
        Some(NeedRecord::Site(site)) => {
            // A site met while its own formula is compiled says nothing of itself: it is taken to cost a table.
            let rank = site_root(code, site, masks).map_or(AskRank::Table, |root| rank_of(masks, root));
            pushed(masks, FormulaNode::Site(site), rank)
        }
        Some(NeedRecord::Table(table)) => pushed(masks, FormulaNode::Table(table), AskRank::Table),
        Some(NeedRecord::All {
            parts: count,
            offsets,
        }) => parts_node(code, masks, count, offsets, true),
        Some(NeedRecord::Any {
            parts: count,
            offsets,
        }) => parts_node(code, masks, count, offsets, false),
    };
    let at_place = &mut masks.formulas.at_place;
    if at_place.len() <= place {
        at_place.resize(place.saturating_add(1), 0);
    }
    if let Some(held) = at_place.get_mut(place) {
        *held = node.saturating_add(1);
    }
    node
}

/// The node of a site's formula, compiled at the site's first ask. None while it is being compiled: a formula that
/// names itself.
pub(crate) fn site_root(code: &ProgramCode, site: NeedSite, masks: &mut NeedMasks) -> Option<u32> {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    match masks.formulas.roots.get(place).copied() {
        Some(COMPILING) => return None,
        Some(held) if held > 0 => return Some(held.saturating_sub(1)),
        Some(_) | None => {}
    }
    let roots = &mut masks.formulas.roots;
    if roots.len() <= place {
        roots.resize(place.saturating_add(1), 0);
    }
    if let Some(held) = roots.get_mut(place) {
        *held = COMPILING;
    }
    let node = match slot(code, WordSlot(site.0), WordSlot(0)) {
        Some(formula) => compiled(code, formula, masks),
        None => pushed(masks, FormulaNode::Always, AskRank::Free),
    };
    if let Some(held) = masks.formulas.roots.get_mut(place) {
        *held = node.saturating_add(1);
    }
    Some(node)
}
