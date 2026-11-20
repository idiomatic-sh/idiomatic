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
use crate::domain::lexical_facts::service::{name_word, node, span_at, word};
use crate::domain::lexical_facts::types::NameWords;
use crate::domain::model::flow::types::{FlowContext, FlowEventKind, FlowState, ShadowForm, UseForm};
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

/// Writes one row per binding event, in evaluation order.
pub(crate) fn binding_event_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<BindingEventRow>,
) -> Fill {
    let Some(read) = flow_read(file, rows) else {
        return Fill::Written;
    };
    let mut known = NameWords::default();
    for event in &read.model.flow.events {
        rows.write(BindingEventRow {
            occurrence: node(read, event.at),
            name: name_word(&read.model.names, words, &mut known, event.name),
            scope: node(read, event.scope),
            phase: node(read, event.phase),
            kind: word(words, event_word(event.kind)),
            ordinal: event.ordinal,
            file: read.file,
            span: span_at(read.table, event.at),
            lexical: node(read, event.lexical),
        });
    }
    Fill::Written
}

/// Writes one row per read, in evaluation order: what the read is for and what can reach it.
pub(crate) fn binding_read_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<BindingReadRow>,
) -> Fill {
    let Some(read) = flow_read(file, rows) else {
        return Fill::Written;
    };
    let mut known = NameWords::default();
    for each in &read.model.flow.reads {
        let since = match each.state {
            FlowReadState::Builtin(since) | FlowReadState::DeclarationOrBuiltin(since) => since,
            FlowReadState::Unique
            | FlowReadState::Alternatives
            | FlowReadState::PossiblyUnbound
            | FlowReadState::Unbound
            | FlowReadState::Uncertain => NOT_APPLICABLE,
        };
        rows.write(BindingReadRow {
            reference: node(read, each.at),
            name: name_word(&read.model.names, words, &mut known, each.name),
            scope: node(read, each.scope),
            phase: node(read, each.phase),
            context: word(words, context_word(each.context)),
            state: word(words, state_word(each.state)),
            since_major: since.major,
            since_minor: since.minor,
            file: read.file,
            span: span_at(read.table, each.at),
        });
    }
    Fill::Written
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
    let unique = word(words, NameText(vocabulary::UNIQUE));
    let alternative = word(words, NameText(vocabulary::ALTERNATIVE));
    for each in &read.model.flow.uses {
        rows.write(BindingUseRow {
            reference: node(read, each.at),
            binding: node(read, each.binding),
            scope: node(read, each.scope),
            phase: node(read, each.phase),
            context: word(words, context_word(each.context)),
            certainty: match each.form {
                UseForm::Unique => unique,
                UseForm::Alternative => alternative,
            },
            file: read.file,
            span: span_at(read.table, each.at),
        });
    }
    Fill::Written
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
    let lexical = word(words, NameText(vocabulary::LEXICAL));
    let replacement = word(words, NameText(vocabulary::REPLACEMENT));
    for each in &read.model.flow.shadows {
        rows.write(BindingShadowRow {
            binding: node(read, each.binding),
            previous: node(read, each.previous),
            scope: node(read, each.scope),
            previous_scope: node(read, each.previous_scope),
            phase: node(read, each.phase),
            relationship: match each.form {
                ShadowForm::Lexical => lexical,
                ShadowForm::Replacement => replacement,
            },
            file: read.file,
        });
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
