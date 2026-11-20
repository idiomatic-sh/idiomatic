//! The `binding_facts` service: free functions named by verb that write the rows of the binding relations from the model, with
//! no IO and no ports. Each has the parameters of its relation's filler; the per-file view is whatever implements `ModelView`.
//!
//! The rows are facts of the language: every event, every read with every definition that can reach it. Which event a rule
//! takes for a read (the latest one in schedule order, say) is the rule pack's selection over these rows, not a row of its own:
//! `binding_resolution_rows` writes nothing.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the fillers and the model they read have no caller in this library until the walk's `Fillers` implementation, which delegates to them, joins the branch"
    )
)]

use crate::domain::binding_results::constants as vocabulary;
use crate::domain::lexical_facts::service::{node, span_at};
use crate::domain::model::flow::types::{
    FlowContext, FlowEvent, FlowEventKind, FlowRead, FlowShadow, FlowState, FlowUse, ShadowForm, UseForm,
};
use crate::domain::model::names::service::{held_word, name_word};
use crate::domain::model::names::types::FactWords;
use crate::domain::model::reaching::types::FlowReadState;
use crate::domain::model::view::types::{ModelPart, ModelRead, ModelView};
use crate::domain::qualification::constants::NOT_APPLICABLE;
use ryo::languages::{
    AbsenceCause, BindingEventRow, BindingReadRow, BindingResolutionRow, BindingShadowRow, BindingUseRow, FileWords, Fill,
    Missing, NameText, ReportText, Rows,
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

/// The flow part of the file's model; none, with the relation said partly read, when the flow could not be interpreted.
fn flow_read<'v, V: ModelView, Row>(file: &'v mut V, rows: &mut dyn Rows<Row>) -> Option<ModelRead<'v>> {
    let read = file.semantics(ModelPart::Flow);
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
        kind: held_word(words, &mut known.fixed, event_word(event.kind)),
        ordinal: event.ordinal,
        file: read.file,
        span: span_at(read.table, event.at),
        lexical: node(read, event.lexical),
    }
}

/// Writes one row per binding event, in evaluation order.
pub(crate) fn binding_event_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<BindingEventRow>,
) -> Fill {
    let Some(read) = flow_read(file, rows) else {
        return Fill::Written;
    };
    let mut known = read.model.words.borrow_mut();
    for event in &read.model.flow.events {
        rows.write(event_row(read, words, &mut known, event));
    }
    Fill::Written
}

/// The row of one read: what the read is for and what can reach it.
fn read_row(read: ModelRead<'_>, words: &mut FileWords, known: &mut FactWords, each: &FlowRead) -> BindingReadRow {
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
        context: held_word(words, &mut known.fixed, context_word(each.context)),
        state: held_word(words, &mut known.fixed, state_word(each.state)),
        since_major: since.major,
        since_minor: since.minor,
        file: read.file,
        span: span_at(read.table, each.at),
    }
}

/// Writes one row per read, in evaluation order.
pub(crate) fn binding_read_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<BindingReadRow>,
) -> Fill {
    let Some(read) = flow_read(file, rows) else {
        return Fill::Written;
    };
    let mut known = read.model.words.borrow_mut();
    for each in &read.model.flow.reads {
        rows.write(read_row(read, words, &mut known, each));
    }
    Fill::Written
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
        context: held_word(words, &mut known.fixed, context_word(each.context)),
        certainty: held_word(words, &mut known.fixed, NameText(certainty)),
        file: read.file,
        span: span_at(read.table, each.at),
    }
}

/// Writes one row per definition that can reach a read, in the order of the reads.
pub(crate) fn binding_use_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<BindingUseRow>,
) -> Fill {
    let Some(read) = flow_read(file, rows) else {
        return Fill::Written;
    };
    let mut known = read.model.words.borrow_mut();
    for each in &read.model.flow.uses {
        rows.write(use_row(read, words, &mut known, each));
    }
    Fill::Written
}

/// The row of one binding that shadows an earlier definition of its name.
fn shadow_row(read: ModelRead<'_>, words: &mut FileWords, known: &mut FactWords, each: &FlowShadow) -> BindingShadowRow {
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
        relationship: held_word(words, &mut known.fixed, NameText(relationship)),
        file: read.file,
    }
}

/// Writes one row per binding that shadows an earlier definition of its name, in evaluation order.
pub(crate) fn binding_shadow_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<BindingShadowRow>,
) -> Fill {
    let Some(read) = flow_read(file, rows) else {
        return Fill::Written;
    };
    let mut known = read.model.words.borrow_mut();
    for each in &read.model.flow.shadows {
        rows.write(shadow_row(read, words, &mut known, each));
    }
    Fill::Written
}

/// Writes nothing: which event answers a read is the rule pack's selection over the events, the reads and the scopes.
pub(crate) fn binding_resolution_rows<V: ModelView>(
    _file: &mut V,
    _words: &mut FileWords,
    _rows: &mut dyn Rows<BindingResolutionRow>,
) -> Fill {
    Fill::Absent(AbsenceCause::NotYetRead)
}
