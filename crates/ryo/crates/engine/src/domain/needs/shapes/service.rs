//! The services of the `shapes` part: a shape read back from the pool, and decided over the node table: each star is
//! entered at its kind's elements, the star with the fewest in the file first, and every element of the list is
//! held to the star's field, parent and children. A shape holds when every star has an element that passes.

use super::types::{OwnTree, ShapeCounts, ShapeWords, WrittenNode, WrittenStar};
use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::types::WordSlot;
use crate::domain::needs::types::NeedVerdict;
use ryo_language::domain::node_table::service::{
    children_of, field_of, ids_of_kind, kind_of, own_end, parent_of,
};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_relations::domain::relations::types::{Count, KindBits, Position};

/// The stars of a shape's words (`encoding::needs` writes them); none past a word that cannot be read.
pub(crate) fn stars_of(words: &[WordSlot]) -> Vec<WrittenStar> {
    let mut at = words.iter().copied();
    let node = |at: &mut dyn Iterator<Item = WordSlot>| {
        let kind = at.next()?;
        let field = at.next()?;
        let own = if at.next()?.0 == 1 { OwnTree::Own } else { OwnTree::Any };
        Some(WrittenNode { kind, field, own })
    };
    let mut stars = Vec::new();
    let count = at.next().map_or(0, |count| count.0);
    for _ in 0..count {
        let Some(anchor) = node(&mut at) else { break };
        let parent = match at.next() {
            Some(WordSlot(1)) => node(&mut at),
            Some(_) | None => None,
        };
        let mut children = Vec::new();
        for _ in 0..at.next().map_or(0, |count| count.0) {
            if let Some(child) = node(&mut at) {
                children.push(child);
            }
        }
        stars.push(WrittenStar {
            anchor,
            parent,
            children,
        });
    }
    stars
}

/// What a node of a star asks, in the node table's codes: none for a name the vocabulary does not hold (no element has
/// it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Wanted {
    kind: Option<u16>,
    field: Option<u16>,
    own: OwnTree,
}

/// The code of a constant's word among names; `Some(None)` for no constant; none when no name has it.
fn code_of(words: &ShapeWords<'_>, names: &[ryo_language::domain::facts::types::CellWord], slot: WordSlot) -> Option<Option<u16>> {
    if slot == NO_SLOT {
        return Some(None);
    }
    let word = words.constants.get(usize::try_from(slot.0).unwrap_or(usize::MAX))?;
    let place = names.iter().position(|name| name == word)?;
    Some(Some(u16::try_from(place).ok()?))
}

/// A written node in codes; none when it asks a name no element has.
fn wanted(words: &ShapeWords<'_>, node: WrittenNode) -> Option<Wanted> {
    Some(Wanted {
        kind: code_of(words, words.kinds, node.kind)?,
        field: code_of(words, words.fields, node.field)?,
        own: node.own,
    })
}

/// Whether an element is what a node asks.
fn passes(nodes: &NodeTable, own: Position, element: Position, node: Wanted) -> bool {
    (node.own == OwnTree::Any || element.0 < own.0)
        && node.kind.is_none_or(|kind| kind_of(nodes, element).0 == kind)
        && node.field.is_none_or(|field| field_of(nodes, element).0 == field)
}

/// Whether an element has the neighbourhood of a star: its own field and tree, its parent, a child for each child.
fn neighbourhood(nodes: &NodeTable, own: Position, element: Position, star: &DecidedStar) -> bool {
    passes(nodes, own, element, star.anchor)
        && star
            .parent
            .is_none_or(|parent| passes(nodes, own, parent_of(nodes, element), parent))
        && star.children.iter().all(|child| {
            children_of(nodes, element)
                .iter()
                .any(|at| passes(nodes, own, *at, *child))
        })
}

/// A star in codes, with its kind's elements in the file.
struct DecidedStar {
    anchor: Wanted,
    parent: Option<Wanted>,
    children: Vec<Wanted>,
    kind: KindBits,
}

/// Whether every star of a shape has an element in the file with its neighbourhood: the stars entered in the order of
/// their kinds' element counts in the file (fewest first, then as written), stopping at the first that has none. The
/// first star's elements are all held to it, and counted; the others' stop at the first that passes.
pub(crate) fn shape_held(
    stars: &[WrittenStar],
    nodes: &NodeTable,
    words: &ShapeWords<'_>,
    counts: &mut ShapeCounts,
) -> NeedVerdict {
    let mut decided = Vec::with_capacity(stars.len());
    for star in stars {
        let Some(anchor) = wanted(words, star.anchor) else {
            return NeedVerdict::Empty;
        };
        let Some(kind) = anchor.kind else {
            continue;
        };
        let parent = match star.parent {
            Some(parent) => match wanted(words, parent) {
                Some(parent) => Some(parent),
                None => return NeedVerdict::Empty,
            },
            None => None,
        };
        let mut children = Vec::with_capacity(star.children.len());
        for child in &star.children {
            match wanted(words, *child) {
                Some(child) => children.push(child),
                None => return NeedVerdict::Empty,
            }
        }
        decided.push(DecidedStar {
            anchor,
            parent,
            children,
            kind: KindBits(kind),
        });
    }
    decided.sort_by_key(|star| ids_of_kind(nodes, star.kind).len());
    let own = own_end(nodes);
    for (place, star) in decided.iter().enumerate() {
        let elements = ids_of_kind(nodes, star.kind);
        let found = if place == 0 {
            counts.candidates = Count(counts.candidates.0.saturating_add(elements.len()));
            let passed = elements
                .iter()
                .filter(|element| neighbourhood(nodes, own, **element, star))
                .count();
            counts.survivors = Count(counts.survivors.0.saturating_add(passed));
            passed > 0
        } else {
            elements
                .iter()
                .any(|element| neighbourhood(nodes, own, *element, star))
        };
        if !found {
            return NeedVerdict::Empty;
        }
    }
    NeedVerdict::Holds
}
