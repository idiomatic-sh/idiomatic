// ignore: idiomatic.arch/file-length - one table's writer and readers over its private columns, each call with its example: split, the builder's numbering (nodes, then tokens, then segments) and the readers that rely on it would be read across files
//! The `node_table` service: the calls a language's walk writes one file's table through, and the readers the engine and the
//! fillers read it with. Nodes are numbered as they open; tokens are kept until the nodes of their segment are all numbered and
//! then follow them, so every id a reader sees is final once `finished` answers.

use super::constants::{PENDING, PRESENCE_BITS};
use super::types::{
    BuilderSegment, BuilderState, ByteSpan, ChildPlace, ChildRange, EmbeddedRoot,
    EmbeddedSegment, EmbeddedSpan, EmbeddedText, FieldCode, Finish, Implied, KindSet, KindsPresent,
    LogicalText, NodeTable, NodeTableBuilder, PendingToken, PresenceWord,
};
use crate::domain::diagnostics::constants::UNSPECIFIED_CODE;
use crate::domain::diagnostics::service::{clear, record};
use crate::domain::diagnostics::types::{DiagnosticMetadata, DiagnosticPhase, TreeImpact};
use crate::domain::elements::types::SyntaxAcceptance;
use crate::domain::elements::types::{DecodedProperty, OpenNode};
use ryo_relations::domain::elements::types::EmbeddedMapping;
use ryo_relations::domain::relations::types::{
    Count, KindBits, Offset, Position, PropertyId, ReportText, Span,
};
use ryo_relations::domain::terms::types::{ChildOrdinal, NameText};

/// The id after `count` elements: the next one's.
fn id_after(count: Count) -> Position {
    Position(u32::try_from(count.0).unwrap_or(u32::MAX))
}

/// The builder of the table of one file, for a language with `kinds` kinds; the table is cleared first, keeping its room.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, node_count};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(4));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(3));
/// assert_eq!(node_count(finished(&mut writer)), Count(1));
/// ```
pub fn builder(table: &mut NodeTable, kinds: Count) -> NodeTableBuilder<'_> {
    let columns = &mut table.0;
    columns.kind.clear();
    columns.parent.clear();
    columns.subtree_end.clear();
    columns.start.clear();
    columns.end.clear();
    columns.field.clear();
    columns.ordinal.clear();
    columns.child_range.clear();
    columns.children.clear();
    for ids in &mut columns.by_kind {
        ids.clear();
    }
    for ids in &mut columns.by_field {
        ids.clear();
    }
    if columns.by_kind.len() < kinds.0 {
        columns.by_kind.resize_with(kinds.0, Vec::new);
    }
    columns.present.0.clear();
    columns.nodes = Count(0);
    columns.decoded.clear();
    columns.properties.clear();
    columns.texts.0.clear();
    clear(&mut columns.diagnostics);
    columns.acceptance = SyntaxAcceptance::Undetermined;
    columns.embedded.clear();
    columns.implied.clear();
    columns.sets.clear();
    columns.enclosing_sets.clear();
    for ids in &mut columns.enclosing {
        ids.clear();
    }
    NodeTableBuilder(BuilderState {
        table,
        kinds,
        open: Vec::new(),
        written: Vec::new(),
        opened: Vec::new(),
        pending: Vec::new(),
        segment_start: Position(0),
        segment: BuilderSegment::File,
        cursor: Offset(0),
        finished: Finish::Open,
        implied: Vec::new(),
        logical: LogicalText::default(),
        structured: false,
    })
}

/// A builder as [`builder`] makes, that also writes, as each element is numbered, its [`ElementSets`] and, for each of the
/// kind sets `enclosing`, the nearest strict ancestor whose kind is in it: what [`element_sets`] and [`enclosing_ids`] read.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{element_sets, enclosing_ids, enter_node, finished, leave_node, structured};
/// use ryo_language::domain::node_table::types::{FieldCode, KindSet, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = structured(&mut table, Count(4), &[KindSet([0b10, 0, 0, 0])]);
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// enter_node(&mut writer, KindBits(2), FieldCode(3), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// leave_node(&mut writer, Offset(0));
/// let read = finished(&mut writer);
/// assert_eq!(element_sets(read)[0].child_kinds[0], 0b100);
/// assert_eq!(element_sets(read)[1].parent_kind, 1);
/// assert_eq!(enclosing_ids(read)[0], [u32::MAX, 0]);
/// ```
pub fn structured<'t>(table: &'t mut NodeTable, kinds: Count, enclosing: &[KindSet]) -> NodeTableBuilder<'t> {
    let mut made = builder(table, kinds);
    let columns = &mut made.0.table.0;
    columns.enclosing_sets.extend_from_slice(enclosing);
    if columns.enclosing.len() < enclosing.len() {
        columns.enclosing.resize_with(enclosing.len(), Vec::new);
    }
    made.0.structured = true;
    made
}

/// Whether `kind` is in `set`.
fn in_set(set: &KindSet, kind: u16) -> bool {
    set.0
        .get(usize::from(kind) / 64)
        .is_some_and(|word| word >> (kind % 64) & 1 == 1)
}

/// Writes the sets and enclosing ids of the element `id` just numbered, under `parent`, and adds its kind and field to its
/// parent's sets: the parent is numbered before it (a node in pre-order, a token after its segment's nodes, an embedded root
/// after its host).
fn structure_of(columns: &mut super::types::NodeTableEngine, id: Position, parent: Position, kind: KindBits, field: FieldCode) {
    let (i, p) = (usize::from(id), usize::from(parent));
    let root = i == p;
    let parent_kind = if root { kind.0 } else { columns.kind.get(p).copied().unwrap_or(0) };
    columns.sets.push(super::types::ElementSets {
        parent_kind,
        root,
        ..Default::default()
    });
    if !root && let Some(sets) = columns.sets.get_mut(p) {
        if let Some(word) = sets.child_kinds.get_mut(usize::from(kind.0) / 64) {
            *word |= 1 << (kind.0 % 64);
        }
        if let Some(word) = sets.child_fields.get_mut(usize::from(field.0) / 64) {
            *word |= 1 << (field.0 % 64);
        }
    }
    for (set, ids) in columns.enclosing_sets.iter().zip(columns.enclosing.iter_mut()) {
        let up = if root {
            u32::MAX
        } else if in_set(set, parent_kind) {
            parent.0
        } else {
            ids.get(p).copied().unwrap_or(u32::MAX)
        };
        ids.push(up);
    }
}

/// Where a logical offset of the segment being written is in the file's bytes.
fn mapped(state: &BuilderState<'_>, start: Offset, end: Offset) -> ByteSpan {
    match state.segment {
        BuilderSegment::File => ByteSpan { start, end },
        BuilderSegment::Embedded(_, EmbeddedMapping::Exact(content)) => ByteSpan {
            start: Offset(start.0.saturating_add(content.0)),
            end: Offset(end.0.saturating_add(content.0)),
        },
        BuilderSegment::Embedded(host, EmbeddedMapping::WholeHost) => {
            let columns = &state.table.0;
            match (
                columns.start.get(usize::from(host)),
                columns.end.get(usize::from(host)),
            ) {
                (Some(start), Some(end)) => ByteSpan {
                    start: Offset(*start),
                    end: Offset(*end),
                },
                (Some(_) | None, _) => ByteSpan::default(),
            }
        }
    }
}

/// Adds the element `id` of `kind` to its kind's list as it is numbered: ids are numbered in increasing order, so each list
/// stays in id order. The builder made a list for every kind of the language; a code past them makes its own.
fn numbered(
    columns: &mut super::types::NodeTableEngine,
    kind: KindBits,
    field: FieldCode,
    id: Position,
) {
    listed(&mut columns.by_kind, usize::from(kind.0), id);
    listed(&mut columns.by_field, usize::from(field.0), id);
}

/// Adds `id` to the list of `code`, making the lists up to it when there are fewer.
fn listed(lists: &mut Vec<Vec<Position>>, code: usize, id: Position) {
    if let Some(ids) = lists.get_mut(code) {
        ids.push(id);
        return;
    }
    lists.resize_with(code.saturating_add(1), Vec::new);
    if let Some(ids) = lists.get_mut(code) {
        ids.push(id);
    }
}

/// The kinds present: one bit for each kind whose list holds an id, in as many words as the language's kinds and the
/// largest code written need.
fn presence(columns: &mut super::types::NodeTableEngine, kinds: Count) {
    let mut width = kinds.0;
    columns.present.0.clear();
    for (code, ids) in columns.by_kind.iter().enumerate() {
        if ids.is_empty() {
            continue;
        }
        width = width.max(code.saturating_add(1));
        let word = code / PRESENCE_BITS;
        if columns.present.0.len() <= word {
            columns
                .present
                .0
                .resize(word.saturating_add(1), PresenceWord(0));
        }
        if let Some(bits) = columns.present.0.get_mut(word) {
            bits.0 |= PresenceWord(1).0 << (code % PRESENCE_BITS);
        }
    }
    columns
        .present
        .0
        .resize(width.div_ceil(PRESENCE_BITS), PresenceWord(0));
}

/// The parent and ordinal of the next element of the open node, the segment's host for a segment root.
fn placed(state: &mut BuilderState<'_>, id: Position) -> ChildPlace {
    match state.open.last_mut() {
        Some(top) => {
            let ordinal = top.next;
            top.next = ChildOrdinal(ordinal.0.saturating_add(1));
            ChildPlace {
                parent: top.position,
                ordinal,
            }
        }
        None => ChildPlace {
            parent: match state.segment {
                BuilderSegment::File => id,
                BuilderSegment::Embedded(host, _) => host,
            },
            ordinal: ChildOrdinal(0),
        },
    }
}

/// Opens a node of `kind` filling `field` of its parent, starting at `start`; its children follow until [`leave_node`]. Nodes are
/// numbered as they open, from zero in pre-order (an embedded segment's from the id its `embed` answered), so a walk that keeps a
/// typed node per id counts its own.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, kind_of, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(kind_of(finished(&mut writer), Position(0)), KindBits(1));
/// ```
pub fn enter_node(
    builder: &mut NodeTableBuilder<'_>,
    kind: KindBits,
    field: FieldCode,
    start: Offset,
) {
    let state = &mut builder.0;
    let id = id_after(Count(state.table.0.kind.len()));
    if !state.open.is_empty() {
        state.written.push(id);
    }
    let ChildPlace { parent, ordinal } = placed(state, id);
    let span = mapped(state, start, start);
    let columns = &mut state.table.0;
    columns.kind.push(kind.0);
    columns.parent.push(parent.0);
    columns.subtree_end.push(Position(id.0.saturating_add(1)));
    columns.start.push(span.start.0);
    columns.end.push(span.end.0);
    columns.field.push(field.0);
    columns.ordinal.push(ordinal);
    columns.child_range.push(ChildRange::default());
    numbered(columns, kind, field, id);
    if state.structured {
        structure_of(columns, id, parent, kind, field);
    }
    state.open.push(OpenNode {
        position: id,
        next: ChildOrdinal(0),
    });
    state.opened.push(id_after(Count(state.written.len())));
    state.cursor = state.cursor.max(start);
}

/// Opens a node the parser created without a source location: zero wide, at the furthest byte written so far.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_implied_node, finished, leave_node, span_of};
/// use ryo_language::domain::node_table::types::{ByteSpan, FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_implied_node(&mut writer, KindBits(1), FieldCode(0));
/// leave_node(&mut writer, Offset(9));
/// assert_eq!(span_of(finished(&mut writer), Position(0)), ByteSpan::default());
/// ```
pub fn enter_implied_node(builder: &mut NodeTableBuilder<'_>, kind: KindBits, field: FieldCode) {
    let at_cursor = builder.0.cursor;
    let id = id_after(Count(builder.0.table.0.kind.len()));
    enter_node(builder, kind, field, at_cursor);
    builder.0.implied.push(id);
    builder.0.table.0.implied.push(id);
}

/// Closes the innermost open node, which ends at `end`, and records where its subtree ends. A segment's root closing ends the
/// segment: its tokens are numbered after its nodes.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, subtree_end_of};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(1));
/// leave_node(&mut writer, Offset(2));
/// leave_node(&mut writer, Offset(3));
/// assert_eq!(subtree_end_of(finished(&mut writer), Position(0)), Position(2));
/// ```
pub fn leave_node(builder: &mut NodeTableBuilder<'_>, end: Offset) {
    leave_open(&mut builder.0, end);
}

/// Closes the innermost open node of `state` at `end`.
fn leave_open(state: &mut BuilderState<'_>, end: Offset) {
    let Some(open) = state.open.pop() else {
        return;
    };
    let after = id_after(Count(state.table.0.kind.len()));
    let implied = state.implied.last() == Some(&open.position) && state.implied.pop().is_some();
    let span_end = mapped(state, end, end).end;
    let columns = &mut state.table.0;
    // The node's children, as they were written: its block of `children`; a token's place is kept for its id.
    let from = state
        .opened
        .pop()
        .map_or(state.written.len(), usize::from);
    let first = id_after(Count(columns.children.len()));
    for child in state.written.get(from..).unwrap_or_default() {
        if child.0 & PENDING == 0 {
            columns.children.push(*child);
        } else {
            if let Some(token) = state.pending.get_mut(usize::from(Position(child.0 & !PENDING))) {
                token.slot = id_after(Count(columns.children.len()));
            }
            columns.children.push(Position(0));
        }
    }
    state.written.truncate(from);
    if let Some(range) = columns.child_range.get_mut(usize::from(open.position)) {
        *range = ChildRange {
            from: first,
            to: id_after(Count(columns.children.len())),
        };
    }
    if let Some(subtree) = columns.subtree_end.get_mut(usize::from(open.position)) {
        *subtree = after;
    }
    if !implied
        && let (Some(start), Some(stop)) = (
            columns.start.get(usize::from(open.position)),
            columns.end.get_mut(usize::from(open.position)),
        )
    {
        *stop = span_end.0.max(*start);
    }
    state.cursor = state.cursor.max(end);
    if state.open.is_empty() {
        close_segment(state);
    }
}

/// Numbers the pending tokens of the segment after its nodes, and records an embedded segment.
fn close_segment(state: &mut BuilderState<'_>) {
    let columns = &mut state.table.0;
    let tokens_from = columns.kind.len();
    if state.segment == BuilderSegment::File {
        columns.nodes = Count(tokens_from);
    }
    for token in state.pending.drain(..) {
        let id = id_after(Count(columns.kind.len()));
        columns.kind.push(token.kind.0);
        columns.parent.push(token.parent.0);
        columns.subtree_end.push(Position(id.0.saturating_add(1)));
        columns.start.push(token.span.start.0);
        columns.end.push(token.span.end.0);
        columns.field.push(token.field.0);
        columns.ordinal.push(token.ordinal);
        columns.child_range.push(ChildRange::default());
        numbered(columns, token.kind, token.field, id);
        if state.structured {
            structure_of(columns, id, token.parent, token.kind, token.field);
        }
        if let Some(slot) = columns.children.get_mut(usize::from(token.slot)) {
            *slot = id;
        }
        if let Some(text) = token.decoded {
            columns.decoded.push(DecodedProperty {
                element: id,
                property: PropertyId::default(),
                from: text.start,
                to: text.end,
            });
        }
    }
    if let BuilderSegment::Embedded(host, mapping) = state.segment {
        columns.embedded.push(EmbeddedSegment {
            host,
            root: state.segment_start,
            end: id_after(Count(columns.kind.len())),
            mapping,
        });
    }
    state.segment = BuilderSegment::File;
    state.segment_start = id_after(Count(columns.kind.len()));
}

/// Appends a token of `kind`, filling `field` of the open node, from `start` to `end`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, children_of, enter_node, finished, leave_node, token};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(3));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// token(&mut writer, KindBits(2), FieldCode(0), Offset(0), Offset(1));
/// leave_node(&mut writer, Offset(1));
/// assert_eq!(children_of(finished(&mut writer), Position(0)), &[Position(1)]);
/// ```
pub fn token(
    builder: &mut NodeTableBuilder<'_>,
    kind: KindBits,
    field: FieldCode,
    start: Offset,
    end: Offset,
) {
    pend(builder, kind, field, start, end, None);
}

/// Appends a token as [`token`] does, with the text `decoded` its library read from it: what `decoded_of` answers for it.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, decoded_of, decoded_token, enter_node, finished, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(3));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// decoded_token(&mut writer, KindBits(2), FieldCode(0), Offset(0), Offset(4), NameText("a"));
/// leave_node(&mut writer, Offset(4));
/// assert_eq!(decoded_of(finished(&mut writer), Position(1)), Some(NameText("a")));
/// ```
pub fn decoded_token(
    builder: &mut NodeTableBuilder<'_>,
    kind: KindBits,
    field: FieldCode,
    start: Offset,
    end: Offset,
    decoded: NameText<'_>,
) {
    let texts = &mut builder.0.table.0.texts.0;
    let from = Offset(u32::try_from(texts.len()).unwrap_or(u32::MAX));
    texts.extend_from_slice(decoded.0.as_bytes());
    let to = Offset(u32::try_from(texts.len()).unwrap_or(u32::MAX));
    pend(
        builder,
        kind,
        field,
        start,
        end,
        Some(Span {
            start: from,
            end: to,
        }),
    );
}

/// Keeps a token until the nodes of its segment are numbered. A token of an embedded segment keeps its logical text as its
/// decoded text, so a reader of its text reads what the language read.
fn pend(
    builder: &mut NodeTableBuilder<'_>,
    kind: KindBits,
    field: FieldCode,
    start: Offset,
    end: Offset,
    decoded: Option<Span>,
) {
    let state = &mut builder.0;
    let Some(top) = state.open.last_mut() else {
        return;
    };
    let ordinal = top.next;
    top.next = ChildOrdinal(ordinal.0.saturating_add(1));
    let parent = top.position;
    let span = mapped(state, start, end);
    let decoded = decoded.or_else(|| logical_text(state, start, end));
    state.written.push(Position(
        PENDING | u32::try_from(state.pending.len()).unwrap_or(u32::MAX),
    ));
    state.pending.push(PendingToken {
        kind,
        field,
        parent,
        ordinal,
        span,
        decoded,
        slot: Position(u32::MAX),
    });
    state.cursor = state.cursor.max(end);
}

/// The logical text of an embedded token, kept in the table's texts; nothing for a token of the file's own tree.
fn logical_text(state: &mut BuilderState<'_>, start: Offset, end: Offset) -> Option<Span> {
    let BuilderSegment::Embedded(..) = state.segment else {
        return None;
    };
    let source = state.logical.0.as_bytes();
    let piece = source.get(
        usize::try_from(start.0).unwrap_or(usize::MAX)
            ..usize::try_from(end.0).unwrap_or(usize::MAX),
    )?;
    let texts = &mut state.table.0.texts.0;
    let from = Offset(u32::try_from(texts.len()).unwrap_or(u32::MAX));
    texts.extend_from_slice(piece);
    let to = Offset(u32::try_from(texts.len()).unwrap_or(u32::MAX));
    Some(Span {
        start: from,
        end: to,
    })
}

/// Records a scalar parser property on the open node. Replacing a property replaces its value.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, node_property, property_of};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position, PropertyId};
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// node_property(&mut writer, PropertyId(3), NameText("u8"));
/// leave_node(&mut writer, Offset(2));
/// assert_eq!(property_of(finished(&mut writer), Position(0), PropertyId(3)), Some(NameText("u8")));
/// ```
pub fn node_property(
    builder: &mut NodeTableBuilder<'_>,
    property: PropertyId,
    value: NameText<'_>,
) {
    let written = node_property_fmt(builder, property, format_args!("{}", value.0));
    debug_assert!(
        written.is_ok(),
        "a borrowed string formats into the table's texts without failure"
    );
}

/// Formats a scalar parser property on the open node directly into the table's texts, without a temporary string. A formatter
/// failure leaves the property and the texts unchanged.
///
/// # Errors
///
/// The error of a supplied formatter; the table's writer itself never fails.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, node_property_fmt, property_of};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position, PropertyId};
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// assert!(node_property_fmt(&mut writer, PropertyId(1), format_args!("{}", 7)).is_ok());
/// leave_node(&mut writer, Offset(2));
/// assert_eq!(property_of(finished(&mut writer), Position(0), PropertyId(1)), Some(NameText("7")));
/// ```
pub fn node_property_fmt(
    builder: &mut NodeTableBuilder<'_>,
    property: PropertyId,
    value: std::fmt::Arguments<'_>,
) -> std::fmt::Result {
    use std::io::Write as _;
    let state = &mut builder.0;
    let Some(open) = state.open.last() else {
        return Ok(());
    };
    let element = open.position;
    let columns = &mut state.table.0;
    let checkpoint = columns.texts.0.len();
    if columns.texts.0.write_fmt(value).is_err() {
        columns.texts.0.truncate(checkpoint);
        return Err(std::fmt::Error);
    }
    let record = DecodedProperty {
        element,
        property,
        from: Offset(u32::try_from(checkpoint).unwrap_or(u32::MAX)),
        to: Offset(u32::try_from(columns.texts.0.len()).unwrap_or(u32::MAX)),
    };
    let records = &mut columns.properties;
    match records.binary_search_by_key(&(element, property), |each| (each.element, each.property)) {
        Ok(place) => {
            if let Some(slot) = records.get_mut(place) {
                *slot = record;
            }
        }
        Err(place) => records.insert(place, record),
    }
    Ok(())
}

/// Records what the parser reported between `start` and `end`, as a parse diagnostic that leaves the tree incomplete.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, diagnose, enter_node, finished, leave_node, syntax_error_count};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// diagnose(&mut writer, Offset(0), Offset(1), NameText("unexpected token"));
/// leave_node(&mut writer, Offset(1));
/// assert_eq!(syntax_error_count(finished(&mut writer)), Count(1));
/// ```
pub fn diagnose(
    builder: &mut NodeTableBuilder<'_>,
    start: Offset,
    end: Offset,
    message: NameText<'_>,
) {
    diagnose_with_metadata(
        builder,
        start,
        end,
        message,
        DiagnosticMetadata {
            phase: DiagnosticPhase::Parse,
            impact: TreeImpact::Incomplete,
            code: UNSPECIFIED_CODE,
        },
    );
}

/// Records a diagnostic with its stage, its stable code and its effect on the tree.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::diagnostics::types::{DiagnosticCode, DiagnosticMetadata, DiagnosticPhase, TreeImpact};
/// use ryo_language::domain::node_table::service::{builder, diagnose_with_metadata, enter_node, finished, leave_node, syntax_error_count};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// let metadata = DiagnosticMetadata { phase: DiagnosticPhase::Validation, impact: TreeImpact::Preserved, code: DiagnosticCode("E1") };
/// diagnose_with_metadata(&mut writer, Offset(0), Offset(1), NameText("late"), metadata);
/// leave_node(&mut writer, Offset(1));
/// assert_eq!(syntax_error_count(finished(&mut writer)), Count(1));
/// ```
pub fn diagnose_with_metadata(
    builder: &mut NodeTableBuilder<'_>,
    start: Offset,
    end: Offset,
    message: NameText<'_>,
    metadata: DiagnosticMetadata,
) {
    record(
        &mut builder.0.table.0.diagnostics,
        metadata,
        ReportText(message.0),
        start,
        end,
    );
}

/// Records whether the language's parser accepted the file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::elements::types::SyntaxAcceptance;
/// use ryo_language::domain::node_table::service::{acceptance_of, builder, finished, set_syntax_acceptance};
/// use ryo_language::domain::node_table::types::NodeTable;
/// use ryo_relations::domain::relations::types::Count;
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(1));
/// set_syntax_acceptance(&mut writer, SyntaxAcceptance::Rejected);
/// assert_eq!(acceptance_of(finished(&mut writer)), SyntaxAcceptance::Rejected);
/// ```
pub fn set_syntax_acceptance(builder: &mut NodeTableBuilder<'_>, acceptance: SyntaxAcceptance) {
    builder.0.table.0.acceptance = acceptance;
}

/// Begins an embedded segment over the node `host`, after the file's own tree has closed: the node calls that follow write the
/// segment in the logical offsets of `source`, until its root closes. Answers the id its root takes.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, embed, enter_node, finished, leave_node, parent_of, token};
/// use ryo_language::domain::node_table::types::{EmbeddedRoot, EmbeddedText, FieldCode, NodeTable};
/// use ryo_relations::domain::elements::types::EmbeddedMapping;
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(3));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// token(&mut writer, KindBits(2), FieldCode(0), Offset(0), Offset(3));
/// leave_node(&mut writer, Offset(3));
/// let text = EmbeddedText { text: NameText("x"), mapping: EmbeddedMapping::Exact(Offset(1)) };
/// assert_eq!(embed(&mut writer, Position(0), text), EmbeddedRoot(Position(2)));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(1));
/// assert_eq!(parent_of(finished(&mut writer), Position(2)), Position(0));
/// ```
pub fn embed(
    builder: &mut NodeTableBuilder<'_>,
    host: Position,
    source: EmbeddedText<'_>,
) -> EmbeddedRoot {
    let state = &mut builder.0;
    debug_assert!(
        state.open.is_empty(),
        "an embedded segment begins after the file's own tree has closed"
    );
    let root = id_after(Count(state.table.0.kind.len()));
    state.segment = BuilderSegment::Embedded(host, source.mapping);
    state.segment_start = root;
    state.logical.0.clear();
    state.logical.0.push_str(source.text.0);
    EmbeddedRoot(root)
}

/// Completes the table the walk wrote and lends it for reading: every column, the children and the kind buckets are written as
/// the walk writes, so only nodes a walk left open are closed here. Writing after it is a fault of the walk, and is ignored.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, element_count, enter_node, finished, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(element_count(finished(&mut writer)), Count(1));
/// ```
pub fn finished<'b>(builder: &'b mut NodeTableBuilder<'_>) -> &'b NodeTable {
    let state = &mut builder.0;
    if state.finished == Finish::Open {
        // A walk that stopped early leaves nodes open: they close at the furthest byte written.
        while !state.open.is_empty() {
            let end = state.cursor;
            leave_open(state, end);
        }
        presence(&mut state.table.0, state.kinds);
        state.finished = Finish::Done;
    }
    state.table
}

/// The kind of an element.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, kind_of, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(8));
/// enter_node(&mut writer, KindBits(7), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(kind_of(finished(&mut writer), Position(0)), KindBits(7));
/// ```
#[must_use]
pub fn kind_of(table: &NodeTable, id: Position) -> KindBits {
    KindBits(table.0.kind.get(usize::from(id)).copied().unwrap_or(0))
}

/// The parent of an element: a root's is itself, an embedded root's its host.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, parent_of};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(parent_of(finished(&mut writer), Position(0)), Position(0));
/// ```
#[must_use]
pub fn parent_of(table: &NodeTable, id: Position) -> Position {
    table
        .0
        .parent
        .get(usize::from(id))
        .map_or(id, |parent| Position(*parent))
}

/// Where an element is in the file's bytes.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, span_of};
/// use ryo_language::domain::node_table::types::{ByteSpan, FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(2));
/// leave_node(&mut writer, Offset(5));
/// assert_eq!(span_of(finished(&mut writer), Position(0)), ByteSpan { start: Offset(2), end: Offset(5) });
/// ```
#[must_use]
pub fn span_of(table: &NodeTable, id: Position) -> ByteSpan {
    let columns = &table.0;
    match (
        columns.start.get(usize::from(id)),
        columns.end.get(usize::from(id)),
    ) {
        (Some(start), Some(end)) => ByteSpan {
            start: Offset(*start),
            end: Offset(*end),
        },
        (Some(_) | None, _) => ByteSpan::default(),
    }
}

/// The field an element fills in its parent; zero for none.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, field_of, finished, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// enter_node(&mut writer, KindBits(1), FieldCode(4), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(field_of(finished(&mut writer), Position(1)), FieldCode(4));
/// ```
#[must_use]
pub fn field_of(table: &NodeTable, id: Position) -> FieldCode {
    FieldCode(table.0.field.get(usize::from(id)).copied().unwrap_or(0))
}

/// The place of an element among its parent's children, nodes and tokens merged in source order.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, ordinal_of, token};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
/// use ryo_relations::domain::terms::types::ChildOrdinal;
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(3));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// token(&mut writer, KindBits(2), FieldCode(0), Offset(0), Offset(1));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(1));
/// leave_node(&mut writer, Offset(2));
/// leave_node(&mut writer, Offset(2));
/// assert_eq!(ordinal_of(finished(&mut writer), Position(1)), ChildOrdinal(1));
/// ```
#[must_use]
pub fn ordinal_of(table: &NodeTable, id: Position) -> ChildOrdinal {
    table
        .0
        .ordinal
        .get(usize::from(id))
        .copied()
        .unwrap_or(ChildOrdinal(0))
}

/// The id after the last descendant node of a node (descendant walks visit nodes only); the id after a token.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, subtree_end_of};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(subtree_end_of(finished(&mut writer), Position(0)), Position(1));
/// ```
#[must_use]
pub fn subtree_end_of(table: &NodeTable, id: Position) -> Position {
    table
        .0
        .subtree_end
        .get(usize::from(id))
        .copied()
        .unwrap_or(Position(id.0.saturating_add(1)))
}

/// The children of an element, nodes and tokens merged in source order (an embedded root is not among its host's children).
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, children_of, enter_node, finished, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert!(children_of(finished(&mut writer), Position(0)).is_empty());
/// ```
#[must_use]
pub fn children_of(table: &NodeTable, id: Position) -> &[Position] {
    let range = table
        .0
        .child_range
        .get(usize::from(id))
        .copied()
        .unwrap_or_default();
    table
        .0
        .children
        .get(usize::from(range.from)..usize::from(range.to))
        .unwrap_or_default()
}

/// The ids of every element of `kind`, in id order.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, ids_of_kind, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(3));
/// enter_node(&mut writer, KindBits(2), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(ids_of_kind(finished(&mut writer), KindBits(2)), &[Position(0)]);
/// ```
#[must_use]
pub fn ids_of_kind(table: &NodeTable, kind: KindBits) -> &[Position] {
    table
        .0
        .by_kind
        .get(usize::from(kind.0))
        .map_or(&[], Vec::as_slice)
}

/// The ids of every element that fills `field` in its parent, in id order; field zero lists the elements that fill none.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, ids_of_field, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// enter_node(&mut writer, KindBits(1), FieldCode(3), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(ids_of_field(finished(&mut writer), FieldCode(3)), &[Position(1)]);
/// ```
#[must_use]
pub fn ids_of_field(table: &NodeTable, field: FieldCode) -> &[Position] {
    table
        .0
        .by_field
        .get(usize::from(field.0))
        .map_or(&[], Vec::as_slice)
}

/// The kind code of every element, by id: the column itself, for an engine that reads it whole.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, kind_codes, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(8));
/// enter_node(&mut writer, KindBits(7), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(kind_codes(finished(&mut writer)), &[7]);
/// ```
#[must_use]
pub fn kind_codes(table: &NodeTable) -> &[u16] {
    &table.0.kind
}

/// The field code of every element, by id (zero: none): the column itself.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, field_codes, finished, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(field_codes(finished(&mut writer)), &[0]);
/// ```
#[must_use]
pub fn field_codes(table: &NodeTable) -> &[u16] {
    &table.0.field
}

/// The parent id of every element, by id (a root's is itself): the column itself.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, parent_ids};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(parent_ids(finished(&mut writer)), &[0, 0]);
/// ```
#[must_use]
pub fn parent_ids(table: &NodeTable) -> &[u32] {
    &table.0.parent
}

/// The first byte of every element, by id: the column itself.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, starts};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(2));
/// leave_node(&mut writer, Offset(5));
/// assert_eq!(starts(finished(&mut writer)), &[2]);
/// ```
#[must_use]
pub fn starts(table: &NodeTable) -> &[u32] {
    &table.0.start
}

/// The byte after the last of every element, by id: the column itself.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, ends, enter_node, finished, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(2));
/// leave_node(&mut writer, Offset(5));
/// assert_eq!(ends(finished(&mut writer)), &[5]);
/// ```
#[must_use]
pub fn ends(table: &NodeTable) -> &[u32] {
    &table.0.end
}

/// The sets of every element, by id, as a `structured` builder wrote them (empty for any other builder).
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{element_sets, enter_node, finished, leave_node, structured};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = structured(&mut table, Count(2), &[]);
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert!(element_sets(finished(&mut writer))[0].root);
/// ```
#[must_use]
pub fn element_sets(table: &NodeTable) -> &[super::types::ElementSets] {
    &table.0.sets
}

/// The enclosing column of each kind set a `structured` builder was given, in that order: per element, the nearest strict
/// ancestor whose kind is in the set (an embedded root's ancestors are its host's and the host), `u32::MAX` for none.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{enclosing_ids, enter_node, finished, leave_node, structured};
/// use ryo_language::domain::node_table::types::{FieldCode, KindSet, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = structured(&mut table, Count(2), &[KindSet([0b10, 0, 0, 0])]);
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(enclosing_ids(finished(&mut writer))[0], [u32::MAX]);
/// ```
#[must_use]
pub fn enclosing_ids(table: &NodeTable) -> &[Vec<u32>] {
    table.0.enclosing.get(..table.0.enclosing_sets.len()).unwrap_or_default()
}

/// The kinds that occur in the file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, kinds_present, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, KindsPresent, NodeTable, PresenceWord};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(3));
/// enter_node(&mut writer, KindBits(2), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(kinds_present(finished(&mut writer)), &KindsPresent(vec![PresenceWord(0b100)]));
/// ```
#[must_use]
pub fn kinds_present(table: &NodeTable) -> &KindsPresent {
    &table.0.present
}

/// How many syntax nodes the file's own tree has: its tokens are numbered from here.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, node_count, token};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(3));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// token(&mut writer, KindBits(2), FieldCode(0), Offset(0), Offset(1));
/// leave_node(&mut writer, Offset(1));
/// assert_eq!(node_count(finished(&mut writer)), Count(1));
/// ```
#[must_use]
pub fn node_count(table: &NodeTable) -> Count {
    table.0.nodes
}

/// How many elements the table has: nodes, tokens and embedded segments.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, element_count, enter_node, finished, leave_node, token};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(3));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// token(&mut writer, KindBits(2), FieldCode(0), Offset(0), Offset(1));
/// leave_node(&mut writer, Offset(1));
/// assert_eq!(element_count(finished(&mut writer)), Count(2));
/// ```
#[must_use]
pub fn element_count(table: &NodeTable) -> Count {
    Count(table.0.kind.len())
}

/// A text the table keeps, by its record.
fn text_of<'t>(table: &'t NodeTable, record: &DecodedProperty) -> NameText<'t> {
    NameText(
        table
            .0
            .texts
            .0
            .get(usize::from(Position(record.from.0))..usize::from(Position(record.to.0)))
            .map(|bytes| std::str::from_utf8(bytes).unwrap_or_default())
            .unwrap_or_default(),
    )
}

/// The decoded text of a token, or the logical text of an embedded token; nothing for any other element.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, decoded_of, enter_node, finished, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(decoded_of(finished(&mut writer), Position(0)), None);
/// ```
#[must_use]
pub fn decoded_of(table: &NodeTable, id: Position) -> Option<NameText<'_>> {
    let records = &table.0.decoded;
    match records.binary_search_by_key(&id, |each| each.element) {
        Ok(place) => records.get(place).map(|record| text_of(table, record)),
        Err(_) => None,
    }
}

/// A scalar property the parser recorded on a node.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, property_of};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position, PropertyId};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(property_of(finished(&mut writer), Position(0), PropertyId(1)), None);
/// ```
#[must_use]
pub fn property_of(table: &NodeTable, id: Position, property: PropertyId) -> Option<NameText<'_>> {
    let records = &table.0.properties;
    match records.binary_search_by_key(&(id, property), |each| (each.element, each.property)) {
        Ok(place) => records.get(place).map(|record| text_of(table, record)),
        Err(_) => None,
    }
}

/// Whether the parser implied the node `id`: a node written through `enter_implied_node`. Every other node, every token and an
/// id the table does not hold answer `Implied::No`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_implied_node, enter_node, finished, implied_of, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, Implied, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// enter_implied_node(&mut writer, KindBits(1), FieldCode(0));
/// leave_node(&mut writer, Offset(0));
/// leave_node(&mut writer, Offset(3));
/// let read = finished(&mut writer);
/// assert_eq!(implied_of(read, Position(0)), Implied::No);
/// assert_eq!(implied_of(read, Position(1)), Implied::Yes);
/// ```
#[must_use]
pub fn implied_of(table: &NodeTable, id: Position) -> Implied {
    match table.0.implied.binary_search(&id) {
        Ok(_) => Implied::Yes,
        Err(_) => Implied::No,
    }
}

/// Whether the language's parser accepted the file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::elements::types::SyntaxAcceptance;
/// use ryo_language::domain::node_table::service::{acceptance_of, builder, finished};
/// use ryo_language::domain::node_table::types::NodeTable;
/// use ryo_relations::domain::relations::types::Count;
///
/// let mut table = NodeTable::default();
/// assert_eq!(acceptance_of(finished(&mut builder(&mut table, Count(1)))), SyntaxAcceptance::Undetermined);
/// ```
#[must_use]
pub fn acceptance_of(table: &NodeTable) -> SyntaxAcceptance {
    table.0.acceptance
}

/// How many diagnostics the parser reported about the file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, finished, syntax_error_count};
/// use ryo_language::domain::node_table::types::NodeTable;
/// use ryo_relations::domain::relations::types::Count;
///
/// let mut table = NodeTable::default();
/// assert_eq!(syntax_error_count(finished(&mut builder(&mut table, Count(1)))), Count(0));
/// ```
#[must_use]
pub fn syntax_error_count(table: &NodeTable) -> Count {
    Count(table.0.diagnostics.from.len())
}

/// The id after the last element of the file's own tree, its nodes then its tokens: the embedded segments start here. What a
/// relation of the node-table form whose catalog item declares `extent Own` is bounded by.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, element_count, embed, enter_node, finished, leave_node, own_end, token};
/// use ryo_language::domain::node_table::types::{EmbeddedText, FieldCode, NodeTable};
/// use ryo_relations::domain::elements::types::EmbeddedMapping;
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(3));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// token(&mut writer, KindBits(2), FieldCode(0), Offset(0), Offset(3));
/// leave_node(&mut writer, Offset(3));
/// let text = EmbeddedText { text: NameText("x"), mapping: EmbeddedMapping::Exact(Offset(1)) };
/// embed(&mut writer, Position(0), text);
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(1));
/// let table = finished(&mut writer);
/// assert_eq!(own_end(table), Position(2));
/// assert_eq!(element_count(table), Count(3));
/// ```
#[must_use]
pub fn own_end(table: &NodeTable) -> Position {
    table
        .0
        .embedded
        .first()
        .map_or_else(|| id_after(element_count(table)), |first| first.root)
}

/// The embedded segments of a table, in id order: each with its host, its root and the id after its last element.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, embed, embedded_segments, enter_node, finished, leave_node};
/// use ryo_language::domain::node_table::types::{EmbeddedSpan, EmbeddedText, FieldCode, NodeTable};
/// use ryo_relations::domain::elements::types::EmbeddedMapping;
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(3));
/// let text = EmbeddedText { text: NameText("x"), mapping: EmbeddedMapping::Exact(Offset(1)) };
/// embed(&mut writer, Position(0), text);
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// leave_node(&mut writer, Offset(1));
/// let segments: Vec<EmbeddedSpan> = embedded_segments(finished(&mut writer)).collect();
/// assert_eq!(segments, [EmbeddedSpan { host: Position(0), root: Position(1), end: Position(2) }]);
/// ```
pub fn embedded_segments(table: &NodeTable) -> impl Iterator<Item = EmbeddedSpan> + '_ {
    table.0.embedded.iter().map(|each| EmbeddedSpan {
        host: each.host,
        root: each.root,
        end: each.end,
    })
}
