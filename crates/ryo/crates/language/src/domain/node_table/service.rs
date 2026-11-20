// ignore: idiomatic.arch/file-length - one table's writer and readers over its private columns, each call with its example: split, the builder's numbering (nodes, then tokens, then segments) and the readers that rely on it would be read across files
//! The `node_table` service: the calls a language's walk writes one file's table through, and the readers the engine and the
//! fillers read it with. Nodes are numbered as they open; tokens are kept until the nodes of their segment are all numbered and
//! then follow them, so every id a reader sees is final once `finished` answers.

use super::constants::PRESENCE_BITS;
use super::types::{
    BuilderSegment, BuilderState, ByteSpan, ChildPlace, EmbeddedRoot, EmbeddedSegment,
    EmbeddedText, FieldCode, Finish, KindsPresent, LogicalText, NodeTable, NodeTableBuilder,
    PendingToken, PresenceWord,
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
    columns.span.clear();
    columns.field.clear();
    columns.ordinal.clear();
    columns.child_offsets.clear();
    columns.children.clear();
    columns.by_kind.clear();
    columns.kind_offsets.clear();
    columns.present.0.clear();
    columns.nodes = Count(0);
    columns.decoded.clear();
    columns.properties.clear();
    columns.texts.0.clear();
    clear(&mut columns.diagnostics);
    columns.acceptance = SyntaxAcceptance::Undetermined;
    columns.embedded.clear();
    NodeTableBuilder(BuilderState {
        table,
        kinds,
        open: Vec::new(),
        pending: Vec::new(),
        segment_start: Position(0),
        segment: BuilderSegment::File,
        cursor: Offset(0),
        finished: Finish::Open,
        implied: Vec::new(),
        logical: LogicalText::default(),
    })
}

/// Where a logical offset of the segment being written is in the file's bytes.
fn mapped(state: &BuilderState<'_>, start: Offset, end: Offset) -> ByteSpan {
    match state.segment {
        BuilderSegment::File => ByteSpan { start, end },
        BuilderSegment::Embedded(_, EmbeddedMapping::Exact(content)) => ByteSpan {
            start: Offset(start.0.saturating_add(content.0)),
            end: Offset(end.0.saturating_add(content.0)),
        },
        BuilderSegment::Embedded(host, EmbeddedMapping::WholeHost) => state
            .table
            .0
            .span
            .get(usize::from(host))
            .copied()
            .unwrap_or_default(),
    }
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
    let ChildPlace { parent, ordinal } = placed(state, id);
    let span = mapped(state, start, start);
    let columns = &mut state.table.0;
    columns.kind.push(kind);
    columns.parent.push(parent);
    columns.subtree_end.push(Position(id.0.saturating_add(1)));
    columns.span.push(span);
    columns.field.push(field);
    columns.ordinal.push(ordinal);
    state.open.push(OpenNode {
        position: id,
        next: ChildOrdinal(0),
    });
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
    if let Some(subtree) = columns.subtree_end.get_mut(usize::from(open.position)) {
        *subtree = after;
    }
    if !implied && let Some(span) = columns.span.get_mut(usize::from(open.position)) {
        span.end = span_end.max(span.start);
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
        columns.kind.push(token.kind);
        columns.parent.push(token.parent);
        columns.subtree_end.push(Position(id.0.saturating_add(1)));
        columns.span.push(token.span);
        columns.field.push(token.field);
        columns.ordinal.push(token.ordinal);
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
    state.pending.push(PendingToken {
        kind,
        field,
        parent,
        ordinal,
        span,
        decoded,
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

/// Completes the table the walk wrote, the children and kind buckets built, and lends it for reading. Writing after it is a fault
/// of the walk, and is ignored.
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
        index(state.table, state.kinds);
        state.finished = Finish::Done;
    }
    state.table
}

/// Builds the children lists, the kind buckets and the kinds present.
fn index(table: &mut NodeTable, kinds: Count) {
    let columns = &mut table.0;
    let count = columns.kind.len();
    let roots: Vec<Position> = std::iter::once(Position(0))
        .chain(columns.embedded.iter().map(|segment| segment.root))
        .collect();
    // Children: counted by parent, then each placed at its parent's offset plus its ordinal (its place in source order).
    columns.child_offsets.clear();
    columns
        .child_offsets
        .resize(count.saturating_add(1), Position(0));
    for (index, parent) in columns.parent.iter().enumerate() {
        if count > 0 && !roots.contains(&id_after(Count(index))) {
            if let Some(slot) = columns
                .child_offsets
                .get_mut(usize::from(*parent).saturating_add(1))
            {
                slot.0 = slot.0.saturating_add(1);
            }
        }
    }
    for index in 0..count {
        let before = columns
            .child_offsets
            .get(index)
            .copied()
            .unwrap_or(Position(0));
        if let Some(slot) = columns.child_offsets.get_mut(index.saturating_add(1)) {
            slot.0 = slot.0.saturating_add(before.0);
        }
    }
    columns.children.clear();
    columns.children.resize(
        usize::from(
            columns
                .child_offsets
                .get(count)
                .copied()
                .unwrap_or(Position(0)),
        ),
        Position(0),
    );
    for (index, (parent, ordinal)) in columns.parent.iter().zip(&columns.ordinal).enumerate() {
        if roots.contains(&id_after(Count(index))) {
            continue;
        }
        let place = usize::from(
            columns
                .child_offsets
                .get(usize::from(*parent))
                .copied()
                .unwrap_or(Position(0)),
        )
        .saturating_add(usize::try_from(ordinal.0).unwrap_or(usize::MAX));
        if let Some(slot) = columns.children.get_mut(place) {
            *slot = id_after(Count(index));
        }
    }
    // Kind buckets: a counting sort of the ids by kind, in id order within a kind.
    let width = columns
        .kind
        .iter()
        .map(|kind| usize::from(kind.0).saturating_add(1))
        .max()
        .unwrap_or(0)
        .max(kinds.0);
    columns.kind_offsets.clear();
    columns
        .kind_offsets
        .resize(width.saturating_add(1), Position(0));
    for kind in &columns.kind {
        if let Some(slot) = columns
            .kind_offsets
            .get_mut(usize::from(kind.0).saturating_add(1))
        {
            slot.0 = slot.0.saturating_add(1);
        }
    }
    for index in 0..width {
        let before = columns
            .kind_offsets
            .get(index)
            .copied()
            .unwrap_or(Position(0));
        if let Some(slot) = columns.kind_offsets.get_mut(index.saturating_add(1)) {
            slot.0 = slot.0.saturating_add(before.0);
        }
    }
    let mut fill: Vec<Position> = columns.kind_offsets.iter().take(width).copied().collect();
    columns.by_kind.clear();
    columns.by_kind.resize(count, Position(0));
    columns.present.0.clear();
    columns
        .present
        .0
        .resize(width.div_ceil(PRESENCE_BITS), PresenceWord(0));
    for (index, kind) in columns.kind.iter().enumerate() {
        let code = usize::from(kind.0);
        if let Some(next) = fill.get_mut(code) {
            if let Some(slot) = columns.by_kind.get_mut(usize::from(*next)) {
                *slot = id_after(Count(index));
            }
            next.0 = next.0.saturating_add(1);
        }
        if let Some(word) = columns.present.0.get_mut(code / PRESENCE_BITS) {
            word.0 |= PresenceWord(1).0 << (code % PRESENCE_BITS);
        }
    }
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
    table
        .0
        .kind
        .get(usize::from(id))
        .copied()
        .unwrap_or(KindBits(0))
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
    table.0.parent.get(usize::from(id)).copied().unwrap_or(id)
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
    table
        .0
        .span
        .get(usize::from(id))
        .copied()
        .unwrap_or_default()
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
    table
        .0
        .field
        .get(usize::from(id))
        .copied()
        .unwrap_or_default()
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
    let from = table
        .0
        .child_offsets
        .get(usize::from(id))
        .copied()
        .unwrap_or(Position(0));
    let to = table
        .0
        .child_offsets
        .get(usize::from(id).saturating_add(1))
        .copied()
        .unwrap_or(from);
    table
        .0
        .children
        .get(usize::from(from)..usize::from(to))
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
    let code = usize::from(kind.0);
    let from = table
        .0
        .kind_offsets
        .get(code)
        .copied()
        .unwrap_or(Position(0));
    let to = table
        .0
        .kind_offsets
        .get(code.saturating_add(1))
        .copied()
        .unwrap_or(from);
    table
        .0
        .by_kind
        .get(usize::from(from)..usize::from(to))
        .unwrap_or_default()
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
