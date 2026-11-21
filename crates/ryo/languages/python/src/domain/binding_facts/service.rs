//! The `binding_facts` service: free functions named by verb that write the rows of the binding relations from the model, with
//! no IO and no ports. Each writes every row of its relation; the per-file view is whatever implements `ModelView`.
//!
//! The rows are facts of the language: every event, every read with every definition that can reach it, every namespace
//! its name is looked up in and, for `binding-resolution`, the event in effect in each of those namespaces when the file's
//! schedule runs in order (SPIKE ryo-fmx3.1: the rows today's parse-side reader writes, over the model).

use crate::domain::binding_results::constants as vocabulary;
use crate::domain::lexical_facts::service::{node, span_at};
use crate::domain::lookup_path::service::stepped;
use crate::domain::model::flow::types::{
    FlowContext, FlowEvent, FlowEventKind, FlowRead, FlowShadow, FlowState, FlowUse, ScheduleOrder, ShadowForm, UseForm,
};
use crate::domain::model::lexical::constants::MODULE;
use crate::domain::model::lexical::service::{entry, parent_of, scope_at};
use crate::domain::model::lexical::types::{NamespaceKey, NodeClass};
use crate::domain::model::names::service::{fixed_word, name_word};
use crate::domain::model::names::types::FactWords;
use crate::domain::model::reaching::types::FlowReadState;
use crate::domain::model::view::types::{ModelPart, ModelRead, ModelView};
use crate::domain::qualification::constants::NOT_APPLICABLE;
use ryo::languages::{
    BindingEventRow, BindingReadRow, BindingResolutionRow, BindingShadowRow, BindingUseRow, Count, FileWords, Integer,
    LookupScopeRow, Missing, NameText, Position, ReportText, Rows,
};

/// The word of an event's kind.
const fn event_word(kind: FlowEventKind) -> NameText<'static> {
    NameText(match kind {
        FlowEventKind::Wildcard => vocabulary::WILDCARD,
        FlowEventKind::Read => vocabulary::READ,
        FlowEventKind::Bind => vocabulary::BIND,
        FlowEventKind::Annotation => crate::domain::syntax::constants::ANNOTATION_SEED,
        FlowEventKind::Delete => vocabulary::DELETE,
        FlowEventKind::Cleanup => vocabulary::CLEANUP,
        FlowEventKind::Global => vocabulary::GLOBAL,
        FlowEventKind::Nonlocal => vocabulary::NONLOCAL,
    })
}

/// The word of what a read is for.
const fn context_word(context: FlowContext) -> NameText<'static> {
    NameText(match context {
        FlowContext::Value => vocabulary::VALUE,
        FlowContext::Type => vocabulary::TYPE,
        FlowContext::Deletion => vocabulary::DELETION,
    })
}

/// The word of what can reach a read.
const fn state_word(state: FlowReadState) -> NameText<'static> {
    NameText(match state {
        FlowReadState::Unique => vocabulary::UNIQUE_DECLARATION,
        FlowReadState::Alternatives => vocabulary::ALTERNATIVE_DECLARATIONS,
        FlowReadState::PossiblyUnbound => vocabulary::POSSIBLY_UNBOUND,
        FlowReadState::Unbound => vocabulary::UNBOUND,
        FlowReadState::Builtin(_) => vocabulary::BUILTIN,
        FlowReadState::DeclarationOrBuiltin(_) => vocabulary::DECLARATION_OR_BUILTIN,
        FlowReadState::Uncertain => vocabulary::UNCERTAIN,
    })
}

/// The file's model with `part` built, a part that holds the flow; none, with the relation said partly read, when the flow
/// could not be interpreted.
pub(crate) fn flow_read<'v, V: ModelView, Row>(
    file: &'v mut V,
    part: ModelPart,
    rows: &mut dyn Rows<Row>,
) -> Option<ModelRead<'v>> {
    let read = file.semantics(part);
    if read.model.flow.state == FlowState::Incomplete {
        rows.partial(ReportText(vocabulary::INCOMPLETE), Missing::Any);
        return None;
    }
    Some(read)
}

/// The row of one event.
fn event_row(read: ModelRead<'_>, words: &mut FileWords, known: &mut FactWords, event: &FlowEvent) -> BindingEventRow {
    BindingEventRow {
        occurrence: node(read, event.at),
        name: name_word(&read.model.names, words, &mut known.names, event.name),
        scope: node(read, event.scope),
        phase: node(read, event.phase),
        kind: fixed_word(words, &mut known.fixed, event_word(event.kind)),
        ordinal: event.ordinal,
        file: read.file,
        span: span_at(read, event.at),
        lexical: node(read, event.lexical),
    }
}

/// Writes one row per binding event, in evaluation order.
pub(crate) fn binding_event_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<BindingEventRow>,
) {
    let Some(read) = flow_read(file, ModelPart::Flow, rows) else {
        return;
    };
    let mut known = read.model.words.borrow_mut();
    for event in &read.model.flow.events {
        rows.write(event_row(read, words, &mut known, event));
    }
}

/// The row of one read: what the read is for and what can reach it.
pub(crate) fn read_row(
    read: ModelRead<'_>,
    words: &mut FileWords,
    known: &mut FactWords,
    each: &FlowRead,
) -> BindingReadRow {
    let since = match each.state {
        FlowReadState::Builtin(since) | FlowReadState::DeclarationOrBuiltin(since) => since,
        FlowReadState::Unique
        | FlowReadState::Alternatives
        | FlowReadState::PossiblyUnbound
        | FlowReadState::Unbound
        | FlowReadState::Uncertain => NOT_APPLICABLE,
    };
    BindingReadRow {
        reference: node(read, each.at),
        name: name_word(&read.model.names, words, &mut known.names, each.name),
        scope: node(read, each.scope),
        phase: node(read, each.phase),
        context: fixed_word(words, &mut known.fixed, context_word(each.context)),
        state: fixed_word(words, &mut known.fixed, state_word(each.state)),
        since_major: since.major,
        since_minor: since.minor,
        file: read.file,
        span: span_at(read, each.at),
    }
}

/// Writes one row per read, in evaluation order.
pub(crate) fn binding_read_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<BindingReadRow>,
) {
    let Some(read) = flow_read(file, ModelPart::Flow, rows) else {
        return;
    };
    let mut known = read.model.words.borrow_mut();
    for each in &read.model.flow.reads {
        rows.write(read_row(read, words, &mut known, each));
    }
}

/// The row of one definition that can reach a read.
fn use_row(read: ModelRead<'_>, words: &mut FileWords, known: &mut FactWords, each: &FlowUse) -> BindingUseRow {
    let certainty = match each.form {
        UseForm::Unique => vocabulary::UNIQUE,
        UseForm::Alternative => vocabulary::ALTERNATIVE,
    };
    BindingUseRow {
        reference: node(read, each.at),
        binding: node(read, each.binding),
        scope: node(read, each.scope),
        phase: node(read, each.phase),
        context: fixed_word(words, &mut known.fixed, context_word(each.context)),
        certainty: fixed_word(words, &mut known.fixed, NameText(certainty)),
        file: read.file,
        span: span_at(read, each.at),
    }
}

/// Writes one row per definition that can reach a read, in the order of the reads.
pub(crate) fn binding_use_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<BindingUseRow>,
) {
    let Some(read) = flow_read(file, ModelPart::Flow, rows) else {
        return;
    };
    let mut known = read.model.words.borrow_mut();
    for each in &read.model.flow.uses {
        rows.write(use_row(read, words, &mut known, each));
    }
}

/// The row of one binding that shadows an earlier definition of its name.
pub(crate) fn shadow_row(
    read: ModelRead<'_>,
    words: &mut FileWords,
    known: &mut FactWords,
    each: &FlowShadow,
) -> BindingShadowRow {
    let relationship = match each.form {
        ShadowForm::Lexical => vocabulary::LEXICAL,
        ShadowForm::Replacement => vocabulary::REPLACEMENT,
    };
    BindingShadowRow {
        binding: node(read, each.binding),
        previous: node(read, each.previous),
        scope: node(read, each.scope),
        previous_scope: node(read, each.previous_scope),
        phase: node(read, each.phase),
        relationship: fixed_word(words, &mut known.fixed, NameText(relationship)),
        file: read.file,
    }
}

/// Writes one row per binding that shadows an earlier definition of its name, in evaluation order.
pub(crate) fn binding_shadow_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<BindingShadowRow>,
) {
    let Some(read) = flow_read(file, ModelPart::Flow, rows) else {
        return;
    };
    let mut known = read.model.words.borrow_mut();
    for each in &read.model.flow.shadows {
        rows.write(shadow_row(read, words, &mut known, each));
    }
}

/// Writes the rows of the lookup path of one read: one row per namespace its name is looked up in, nearest first.
pub(crate) fn path_rows(
    read: ModelRead<'_>,
    words: &mut FileWords,
    known: &mut FactWords,
    each: &FlowRead,
    rows: &mut dyn Rows<LookupScopeRow>,
) {
    let reference = node(read, each.at);
    let name = name_word(&read.model.names, words, &mut known.names, each.name);
    let context = fixed_word(words, &mut known.fixed, context_word(each.context));
    let span = span_at(read, each.at);
    stepped(read, each, &mut |scope, depth| {
        rows.write(LookupScopeRow { reference, name, scope: node(read, scope), depth, context, file: read.file, span });
    });
}

/// Writes one row per namespace on the lookup path of each read or deletion, in evaluation order, nearest first.
pub(crate) fn lookup_scope_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<LookupScopeRow>,
) {
    let Some(read) = flow_read(file, ModelPart::Flow, rows) else {
        return;
    };
    let mut known = read.model.words.borrow_mut();
    for each in &read.model.flow.reads {
        path_rows(read, words, &mut known, each, rows);
    }
}

/// Whether a node is a type parameter.
fn type_parameter(read: ModelRead<'_>, at: Position) -> bool {
    matches!(
        entry(&read.model.lexical.class, at),
        NodeClass::TypeVar | NodeClass::TypeVarTuple | NodeClass::ParamSpec
    )
}

/// Whether the read at `at` is in a type parameter's bound or default: evaluated after the whole schedule (PEP 695).
fn lazy_read(read: ModelRead<'_>, at: Position) -> bool {
    let mut child = at;
    while child != MODULE {
        let above = parent_of(read.tree, child);
        if above == child {
            return false;
        }
        if type_parameter(read, above) {
            return true;
        }
        child = above;
    }
    false
}

/// Pushes the rows of one read or deletion: for each namespace on its lookup path that holds an event of its name, the latest
/// one of the schedule so far; a type parameter of the name is its own row.
fn resolution_rows_of(
    read: ModelRead<'_>,
    words: &mut FileWords,
    known: &mut FactWords,
    order: &mut ScheduleOrder,
    ordinal: Integer,
    each: &FlowRead,
) {
    let reference = node(read, each.at);
    let context = fixed_word(words, &mut known.fixed, context_word(each.context));
    let span = span_at(read, each.at);
    let events = &read.model.flow.events;
    let ScheduleOrder { latest, rows, .. } = order;
    stepped(read, each, &mut |scope, depth| {
        if type_parameter(read, scope) {
            let id = node(read, scope);
            let kind = fixed_word(words, &mut known.fixed, event_word(FlowEventKind::Bind));
            rows.push((
                ordinal,
                BindingResolutionRow {
                    reference,
                    event: id,
                    kind,
                    scope: id,
                    lexical: id,
                    depth,
                    ordinal: Integer(0),
                    context,
                    file: read.file,
                    span,
                },
            ));
        } else if let Some(found) =
            latest.get(&NamespaceKey { scope, name: each.name }).and_then(|at| events.get(at.0))
        {
            let kind = fixed_word(words, &mut known.fixed, event_word(found.kind));
            rows.push((
                ordinal,
                BindingResolutionRow {
                    reference,
                    event: node(read, found.at),
                    kind,
                    scope: node(read, scope),
                    lexical: node(read, found.lexical),
                    depth,
                    ordinal: found.ordinal,
                    context,
                    file: read.file,
                    span,
                },
            ));
        } else {
            // The namespace holds no event of the name so far: no row.
        }
    });
}

/// Writes, for each read or deletion and each namespace on its lookup path that holds an event of its name, the event in effect
/// there when the file's schedule runs in order: one pass over the events keeps the latest of each name in each namespace. A
/// `global` or `nonlocal` declaration holds for its whole block, so its namespace holds it from the start; the reads of a
/// type parameter's bound or default are answered after the whole schedule. Rows are in the order of the reads, nearest first.
pub(crate) fn binding_resolution_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<BindingResolutionRow>,
) {
    let Some(read) = flow_read(file, ModelPart::Flow, rows) else {
        return;
    };
    let mut known = read.model.words.borrow_mut();
    let mut order = read.model.schedule_order.borrow_mut();
    order.latest.clear();
    order.lazy.clear();
    order.rows.clear();
    let flow = &read.model.flow;
    for (at, event) in flow.events.iter().enumerate() {
        if matches!(event.kind, FlowEventKind::Global | FlowEventKind::Nonlocal) && event.lexical != event.scope {
            let _first = order.latest.entry(NamespaceKey { scope: event.lexical, name: event.name }).or_insert(Count(at));
        }
    }
    let mut cursor = 0_usize;
    for (at, event) in flow.events.iter().enumerate() {
        let key = NamespaceKey { scope: event.scope, name: event.name };
        match event.kind {
            FlowEventKind::Read | FlowEventKind::Delete => {
                let mut found = None;
                while let Some(candidate) = flow.reads.get(cursor) {
                    cursor = cursor.saturating_add(1);
                    if candidate.at == event.at {
                        found = Some(*candidate);
                        break;
                    }
                }
                let fallback = if event.kind == FlowEventKind::Delete { FlowContext::Deletion } else { FlowContext::Value };
                let each = FlowRead {
                    at: event.at,
                    name: event.name,
                    scope: scope_at(&read.model.lexical, event.at),
                    phase: event.phase,
                    context: found.map_or(fallback, |it| it.context),
                    state: found.map_or(FlowReadState::Uncertain, |it| it.state),
                };
                if event.kind == FlowEventKind::Read && lazy_read(read, event.at) {
                    order.lazy.push((Count(at), each));
                } else {
                    resolution_rows_of(read, words, &mut known, &mut order, event.ordinal, &each);
                }
                if event.kind == FlowEventKind::Delete {
                    let _previous = order.latest.insert(key, Count(at));
                }
            }
            FlowEventKind::Bind | FlowEventKind::Annotation | FlowEventKind::Cleanup => {
                let _previous = order.latest.insert(key, Count(at));
            }
            FlowEventKind::Global | FlowEventKind::Nonlocal => {
                let _previous = order.latest.insert(key, Count(at));
                if event.lexical != event.scope {
                    let _declaring =
                        order.latest.insert(NamespaceKey { scope: event.lexical, name: event.name }, Count(at));
                }
            }
            FlowEventKind::Wildcard => {}
        }
    }
    if !order.lazy.is_empty() {
        let lazy = std::mem::take(&mut order.lazy);
        for (at, each) in &lazy {
            let ordinal = flow.events.get(at.0).map_or(Integer(0), |event| event.ordinal);
            resolution_rows_of(read, words, &mut known, &mut order, ordinal, each);
        }
        order.lazy = lazy;
        order.rows.sort_by_key(|(ordinal, row)| (ordinal.0, row.depth.0));
    }
    for (_, row) in &order.rows {
        rows.write(*row);
    }
}
