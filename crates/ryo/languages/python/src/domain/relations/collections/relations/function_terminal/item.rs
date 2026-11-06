//! Read one terminal outcome for every visible function definition.
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    semantic_syntax::types::Visibility,
    syntax::{service::kind_bits, types::Kind},
    terminal_flow::{
        service::{outcome, prepare},
        types::{IndexState, TerminalIndex},
    },
};
use ryo::languages;
use super::constants::INCOMPLETE;

/// Reads one terminal outcome for each visible function.
pub(crate) struct FunctionTerminalItem;

/// Return whether the terminal index covers the current source.
fn state(index: &TerminalIndex) -> IndexState {
    index.state
}

ryo::reads! {
    FunctionTerminalItem => languages::FunctionTerminal, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given index = prepare(tree, &model.terminal_flow);
        when state(&index) {
            IndexState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            IndexState::Fresh => { partial INCOMPLETE => languages::Missing::Any; }
            IndexState::Complete => {}
        }
        each (crate::domain::semantic_syntax::service::positions(tree)) as function {
            require matches!(crate::domain::syntax::service::kind(tree,function), Kind::Node(Node::FunctionDef|Node::AsyncFunctionDef));
            require crate::domain::semantic_syntax::service::visibility(tree, model, function, symbols) == Visibility::Visible;
            given outcome = outcome(&index,function)?;
            given outcome = languages::intern_derived(symbols, tree.source, languages::NameText(outcome));
            row { id: node(function), outcome: value(outcome), file: file, span: span(function), }
        }
    }
}
