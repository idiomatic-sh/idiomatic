//! The services of the `formulas` part: a site's formula compiled from the code's records into nodes, once a worker at
//! the site's first decision. A record met again (a part many formulas share) is compiled once; another site is named,
//! never copied in.

use super::types::{AskRank, FormulaNode};
use crate::domain::code::records::service::need_at;
use crate::domain::code::records::types::NeedRecord;
use crate::domain::code::types::{NeedSite, ProgramCode, WordSlot};
use crate::domain::needs::masks::service::numbered;
use crate::domain::needs::masks::types::{MustWord, NeedMasks, NeedTerm, RecordCost, Stretch};
use crate::domain::steps::types::MadeTableId;
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

/// The most words of a node's musts: past it the words of the lowest places are kept, a part of what it needs.
const MOST_WORDS: usize = 8;

/// The musts of a node.
fn musts_of(masks: &NeedMasks, node: u32) -> &[MustWord] {
    let formulas = &masks.formulas;
    let stretch = formulas
        .node_musts
        .get(usize::try_from(node).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or_default();
    formulas
        .musts
        .get(stretch.start.0..stretch.start.0.saturating_add(stretch.count.0))
        .unwrap_or_default()
}

/// The words of one bit.
fn bit_words(bit: u32) -> Vec<MustWord> {
    let place = usize::try_from(bit).unwrap_or(usize::MAX);
    vec![MustWord {
        word: Count(place / 64),
        bits: 1_u64 << (place % 64),
    }]
}

/// What every one of several nodes needs: the words of all, joined.
fn union(masks: &NeedMasks, parts: &[u32]) -> Vec<MustWord> {
    let mut all: Vec<MustWord> = Vec::new();
    for part in parts {
        for word in musts_of(masks, *part) {
            match all.iter_mut().find(|held| held.word == word.word) {
                Some(held) => held.bits |= word.bits,
                None => all.push(*word),
            }
        }
    }
    all.sort_unstable_by_key(|word| word.word);
    all.truncate(MOST_WORDS);
    all
}

/// What some one of several nodes needs, whichever it is: the bits all of them need.
fn intersection(masks: &NeedMasks, parts: &[u32]) -> Vec<MustWord> {
    let Some((first, rest)) = parts.split_first() else {
        return Vec::new();
    };
    let mut common: Vec<MustWord> = musts_of(masks, *first).to_vec();
    for part in rest {
        let theirs = musts_of(masks, *part);
        for word in &mut common {
            word.bits &= theirs
                .iter()
                .find(|held| held.word == word.word)
                .map_or(0, |held| held.bits);
        }
        common.retain(|word| word.bits != 0);
    }
    common
}

/// A node added, with what asking it may cost and the free terms it cannot hold without; answers its number.
fn pushed(masks: &mut NeedMasks, node: FormulaNode, rank: AskRank, musts: &[MustWord]) -> u32 {
    let formulas = &mut masks.formulas;
    formulas.node_musts.push(Stretch {
        start: Count(formulas.musts.len()),
        count: Count(musts.len()),
    });
    formulas.musts.extend_from_slice(musts);
    formulas.nodes.push(node);
    formulas.ranks.push(rank);
    u32::try_from(formulas.nodes.len().saturating_sub(1)).unwrap_or(u32::MAX)
}

/// The musts of a node, by number, as the file's free bits must hold them: a stretch of the formulas' musts.
pub(crate) fn node_musts(masks: &NeedMasks, node: u32) -> Stretch {
    masks
        .formulas
        .node_musts
        .get(usize::try_from(node).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or_default()
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
        (RecordCost::Free, Some(bit)) => {
            pushed(masks, FormulaNode::Free(bit), AskRank::Free, &bit_words(bit))
        }
        (RecordCost::Presence, _) => pushed(masks, FormulaNode::Term(number), AskRank::Presence, &[]),
        (RecordCost::Unknown | RecordCost::Free | RecordCost::Dear, _) => {
            pushed(masks, FormulaNode::Term(number), AskRank::Row, &[])
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
            None => parts.push(pushed(masks, FormulaNode::Always, AskRank::Free, &[])),
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
    if !words.is_empty() {
        let stretch = Stretch {
            start: Count(masks.formulas.bits.len()),
            count: Count(words.len()),
        };
        masks.formulas.bits.extend_from_slice(&words);
        let one = words.len() == 1 && words.iter().all(|word| word.bits.count_ones() == 1);
        let (node, musts) = if every {
            (FormulaNode::AllBits(stretch), words.clone())
        } else if one {
            (FormulaNode::AnyBits(stretch), words.clone())
        } else {
            (FormulaNode::AnyBits(stretch), Vec::new())
        };
        let bits = pushed(masks, node, AskRank::Free, &musts);
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
        return pushed(masks, node, AskRank::Free, &[]);
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
    let (node, musts) = if every {
        (FormulaNode::All(stretch), union(masks, &parts))
    } else {
        (FormulaNode::Any(stretch), intersection(masks, &parts))
    };
    pushed(masks, node, rank, &musts)
}

/// The node of a table's row: its own need and then its row, so that a need the free terms decide is decided with no
/// table asked.
fn table_node(code: &ProgramCode, masks: &mut NeedMasks, table: MadeTableId) -> u32 {
    let row = pushed(masks, FormulaNode::Table(table), AskRank::Table, &[]);
    let Some(need) = code
        .tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
        .map(|held| held.need)
        .filter(|need| usize::try_from(need.0).is_ok_and(|place| place < code.needs.len()))
    else {
        return row;
    };
    let root = site_root(code, need, masks);
    let rank = root.map_or(AskRank::Table, |root| rank_of(masks, root));
    let musts = root.map_or_else(Vec::new, |root| musts_of(masks, root).to_vec());
    let own = pushed(masks, FormulaNode::TableNeed(table), rank, &musts);
    let stretch = Stretch {
        start: Count(masks.formulas.parts.len()),
        count: Count(2),
    };
    masks.formulas.parts.extend_from_slice(&[own, row]);
    pushed(masks, FormulaNode::All(stretch), AskRank::Table, &musts)
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
        None | Some(NeedRecord::Unknown) => pushed(masks, FormulaNode::Always, AskRank::Free, &[]),
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
            let root = site_root(code, site, masks);
            let rank = root.map_or(AskRank::Table, |root| rank_of(masks, root));
            let musts = root.map_or_else(Vec::new, |root| musts_of(masks, root).to_vec());
            pushed(masks, FormulaNode::Site(site), rank, &musts)
        }
        Some(NeedRecord::Table(table)) => table_node(code, masks, table),
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
        None => pushed(masks, FormulaNode::Always, AskRank::Free, &[]),
    };
    if let Some(held) = masks.formulas.roots.get_mut(place) {
        *held = node.saturating_add(1);
    }
    Some(node)
}
