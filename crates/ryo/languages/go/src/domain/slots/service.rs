//! The `slots` service: free functions named by verb over the domain's types, with no IO and no ports. It tells what an identifier is
//! where it stands, and what an identifier or a selection is as a reference.

use ryo::languages::{Position, Tree};

use super::types::{Referencing, Slot};
use crate::domain::assignments::service::{left, operator};
use crate::domain::fields::service::names as field_names;
use crate::domain::names::service::dottedness;
use crate::domain::names::types::Dottedness;
use crate::domain::nodes::types::Node;
use crate::domain::signatures::service::signature;
use crate::domain::specs::service::{
    import_name as spec_import_name, name as spec_name, names as spec_names,
};
use crate::domain::syntax::service::{kind, nodes, parent};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// The name of the function whose function type is at `function_type`.
fn signature_name(tree: &Tree<'_>, function_type: Position) -> Option<Position> {
    signature(tree, parent(tree, function_type)).name
}

/// Whether the assignment or `range` clause at `at` declares what it assigns to: it uses `:=`.
fn defines(tree: &Tree<'_>, at: Position) -> Option<Position> {
    operator(tree, at)
        .filter(|each| each.token == Token::Define)
        .map(|each| each.at)
}

/// What the identifier at `ident` is where it stands: a name it declares, a use of one, or neither.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the holders that declare a name, select one or label one are told apart; every other holder uses the name"
)]
pub(crate) fn slot_of(tree: &Tree<'_>, ident: Position) -> Slot {
    let holder = parent(tree, ident);
    let first = || nodes(tree, holder).next() == Some(ident);
    match kind(tree, holder) {
        Kind::Node(Node::File | Node::SelectorExpr | Node::LabeledStmt | Node::BranchStmt) => {
            Slot::Elsewhere
        }
        Kind::Node(Node::Field) if field_names(tree, holder).any(|each| each == ident) => {
            Slot::Declares
        }
        Kind::Node(Node::ValueSpec) if spec_names(tree, holder).any(|each| each == ident) => {
            Slot::Declares
        }
        Kind::Node(Node::ImportSpec) if spec_import_name(tree, holder) == Some(ident) => {
            Slot::Declares
        }
        Kind::Node(Node::TypeSpec) if spec_name(tree, holder) == Some(ident) => Slot::Declares,
        Kind::Node(Node::FuncType) if signature_name(tree, holder) == Some(ident) => Slot::Declares,
        Kind::Node(Node::AssignStmt | Node::RangeStmt)
            if defines(tree, holder).is_some() && left(tree, holder).any(|each| each == ident) =>
        {
            Slot::Declares
        }
        Kind::Node(Node::KeyValueExpr)
            if first() && kind(tree, parent(tree, holder)) == Kind::Node(Node::CompositeLit) =>
        {
            Slot::Ambiguous
        }
        _ => Slot::Uses,
    }
}

/// The selection at `at` when it is the longest dotted name there: it is no operand of another selection.
fn longest(tree: &Tree<'_>, at: Position) -> Option<Position> {
    let above = parent(tree, at);
    (!(kind(tree, above) == Kind::Node(Node::SelectorExpr)
        && nodes(tree, above).next() == Some(at)))
    .then_some(at)
}

/// What the element at `at`, an identifier or a selection, is as a reference.
#[must_use]
pub(crate) fn referencing(tree: &Tree<'_>, at: Position) -> Referencing {
    if kind(tree, at) == Kind::Node(Node::SelectorExpr) {
        return if dottedness(tree, at) == Dottedness::Dotted && longest(tree, at).is_some() {
            Referencing::Used
        } else {
            Referencing::Other
        };
    }
    match slot_of(tree, at) {
        Slot::Uses => Referencing::Used,
        Slot::Ambiguous => Referencing::Unclear,
        Slot::Declares | Slot::Elsewhere => Referencing::Other,
    }
}
