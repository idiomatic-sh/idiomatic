//! The services of the `presence` part: a need that asks a relation of the node-table form for a row with a kind or a
//! field is decided by a set test. The kinds that occur are read off the node table's lists by kind; the fields by one
//! pass over the field column, on the first ask of a field in the file. Either set is built once per file and shared by
//! every rule of the run. A rule that consults a decision over the node rows is told to the budget as having read the
//! node table, whichever rule's ask built the set: the charge does not depend on rule order.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::types::{ConstantNumber, NeedSite, RelationNumber, WordSlot};
use crate::domain::executing::budget::service::reach;
use crate::domain::made_tables::types::Reached;
use crate::domain::memos::types::ChargedUnit;
use crate::domain::needs::service::asks_nodes;
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::constant;
use crate::domain::scratch::types::{NodeNeed, NodePresence, PresentNames};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::service::{element_count, field_of, ids_of_kind, own_end};
use ryo_language::domain::node_table::types::{NodeTable, PresenceWord};
use ryo_relations::domain::catalog::service::{node_column, node_extent};
use ryo_relations::domain::relations::types::{Count, KindBits, NodeColumn, NodeExtent, Position};
use ryo_values::domain::plan_parts::service::relation_column;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The word a read of a name column gives for a code: the name's word, or the word of a code the vocabulary states
/// nothing at, as the fill side reads it.
fn word_of(names: &[CellWord], code: Count) -> CellWord {
    match names.get(code.0) {
        Some(word) => *word,
        None => CellWord(u64::MAX),
    }
}

/// The kinds that occur, from the node table's list of each kind: a kind is in the own tree when its first element is.
fn kinds_built(found: &mut PresentNames, nodes: &NodeTable, names: &[CellWord]) {
    let own = own_end(nodes);
    for (code, word) in names.iter().enumerate() {
        let kind = KindBits(u16::try_from(code).unwrap_or(u16::MAX));
        if let Some(first) = ids_of_kind(nodes, kind).first() {
            found.every.push(*word);
            if first.0 < own.0 {
                found.own.push(*word);
            }
        }
    }
}

/// Marks the field codes of the elements from `from` up to `end` in `marks`, sixty-four codes a word.
fn marked(marks: &mut Vec<PresenceWord>, nodes: &NodeTable, from: Position, end: Position) {
    for element in from.0..end.0 {
        let code = usize::from(field_of(nodes, Position(element)).0);
        if marks.len() <= code / 64 {
            marks.resize(code / 64 + 1, PresenceWord(0));
        }
        if let Some(word) = marks.get_mut(code / 64) {
            word.0 |= 1_u64 << (code % 64);
        }
    }
}

/// The words of the codes marked, added to `out`.
fn words_marked(marks: &[PresenceWord], names: &[CellWord], out: &mut Vec<CellWord>) {
    for (place, word) in marks.iter().enumerate() {
        let mut left = word.0;
        while left != 0 {
            let bit = usize::try_from(left.trailing_zeros()).unwrap_or(0);
            out.push(word_of(names, Count(place * 64 + bit)));
            left &= left - 1;
        }
    }
}

/// The fields that occur, by one pass over the field column: the own tree's elements, then the embedded segments'.
fn fields_built(presence: &mut NodePresence, nodes: &NodeTable, names: &[CellWord]) {
    let NodePresence { fields, marks, .. } = presence;
    let (own, all) = (
        own_end(nodes),
        Position(u32::try_from(element_count(nodes).0).unwrap_or(u32::MAX)),
    );
    marks.clear();
    marked(marks, nodes, Position(0), own);
    words_marked(marks, names, &mut fields.own);
    marked(marks, nodes, own, all);
    words_marked(marks, names, &mut fields.every);
}

/// A need holds when what it asks for was found at least once.
const fn held_when(found: Count) -> NeedVerdict {
    if found.0 > 0 {
        NeedVerdict::Holds
    } else {
        NeedVerdict::Empty
    }
}

/// Whether a relation of the node-table form may have a row whose kind or field column holds a constant, in this file:
/// a set test. None when the need is not of that shape (a filled relation, or another column), which is decided as
/// before. With no column, the relation has a row when its extent has an element.
pub(crate) fn node_row(
    frame: &mut StepFrame<'_, '_>,
    relation: RelationNumber,
    column: WordSlot,
    value: WordSlot,
) -> Option<NeedVerdict> {
    let programs = frame.fill.programs;
    let read = programs
        .code
        .relations
        .get(usize::try_from(relation.0).unwrap_or(usize::MAX))
        .copied()?;
    let extent = node_extent(read)?;
    let nodes = frame.fill.nodes;
    if column == NO_SLOT || value == NO_SLOT {
        let elements = match extent {
            NodeExtent::Own => usize::try_from(own_end(nodes).0).unwrap_or(usize::MAX),
            NodeExtent::Embedded => element_count(nodes).0,
        };
        return Some(held_when(Count(elements)));
    }
    let named = relation_column(read, ColumnIndex(column.0)).and_then(node_column)?;
    let wanted = constant(frame, ConstantNumber(value.0));
    let (stamp, presence) = (frame.fill.store.stamp, &mut frame.state.presence);
    let names = match named {
        NodeColumn::Kind => {
            if presence.kinds.file != Some(stamp) {
                presence.kinds.own.clear();
                presence.kinds.every.clear();
                kinds_built(&mut presence.kinds, nodes, programs.kinds);
                presence.kinds.own.sort_unstable();
                presence.kinds.every.sort_unstable();
                presence.kinds.file = Some(stamp);
            }
            &presence.kinds
        }
        NodeColumn::Field => {
            if presence.fields.file != Some(stamp) {
                presence.fields.own.clear();
                presence.fields.every.clear();
                fields_built(presence, nodes, programs.fields);
                presence.fields.own.sort_unstable();
                presence.fields.every.sort_unstable();
                presence.fields.file = Some(stamp);
            }
            &presence.fields
        }
        NodeColumn::Id
        | NodeColumn::Parent
        | NodeColumn::Ordinal
        | NodeColumn::Start
        | NodeColumn::End
        | NodeColumn::Span
        | NodeColumn::File
        | NodeColumn::Text => return None,
    };
    let present = match extent {
        NodeExtent::Own => &names.own,
        NodeExtent::Embedded => &names.every,
    };
    Some(held_when(Count(usize::from(
        present.binary_search(&wanted).is_ok(),
    ))))
}

/// A rule consulted the decision of `site`: when its formula asks the node rows, the rule read the node table, whether
/// this ask decided it or an earlier rule's did. Whether a site's formula asks is found once and kept.
pub(crate) fn consulted(frame: &mut StepFrame<'_, '_>, site: NeedSite) {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    let kept = &mut frame.state.node_needs;
    if kept.len() <= place {
        kept.resize(place.saturating_add(1), None);
    }
    let asked = match kept.get(place).copied().flatten() {
        Some(known) => known,
        None => {
            let found = match asks_nodes(frame.fill.programs.code, site) {
                NeedVerdict::Holds => NodeNeed::Asked,
                NeedVerdict::Empty => NodeNeed::NotAsked,
            };
            if let Some(slot) = frame.state.node_needs.get_mut(place) {
                *slot = Some(found);
            }
            found
        }
    };
    if asked == NodeNeed::Asked {
        reach(frame, ChargedUnit::Fill(Reached::Nodes));
    }
}
