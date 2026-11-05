//! The `inherits` relation of the language: each type embedded in a struct and each interface embedded in an interface, by the dotted
//! name it writes, without its pointer and its type arguments. A union or a `~T` in an interface is a constraint, and no base.

use crate::domain::fields::service::{names, type_of};
use crate::domain::names::service::base_of;
use crate::domain::nodes::types::GoNode;
use crate::domain::signatures::service::listing;
use crate::domain::signatures::types::Listing;
use crate::domain::syntax::service::{ancestors, kind, kind_bits, parent};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `inherits` relation to, which reads its rows from a file's tree.
pub(crate) struct InheritsItem;

/// The type declaration that holds the embedded field at `field`, or the struct or interface type that does.
fn declaration(tree: &languages::Tree<'_>, field: languages::Position) -> languages::Position {
    let list = parent(tree, field);
    ancestors(tree, field)
        .find(|each| kind(tree, *each) == Kind::Node(GoNode::TypeSpec))
        .unwrap_or_else(|| parent(tree, list))
}

ryo::reads! {
    InheritsItem => languages::Inherits |tree, at, symbols| {
        kinds: [Kind::Node(GoNode::Field)] as kind_bits,
        require matches!(listing(tree, parent(tree, at)), Some(Listing::Members | Listing::Methods));
        require names(tree, at).next().is_none();
        given base = type_of(tree, at)?;
        given target = base_of(tree, symbols, base)?;
        row {
            occurrence: node(base),
            declaration: node(declaration(tree, at)),
            target: value(target),
            file: file,
            span: span(base),
        }
    }
}
