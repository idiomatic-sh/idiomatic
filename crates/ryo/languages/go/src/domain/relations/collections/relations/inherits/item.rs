//! The `inherits` relation of the language: each type embedded in a struct and each interface embedded in an interface, by the dotted
//! name it writes, without its pointer and its type arguments. A union or a `~T` in an interface is a constraint, and no base.

use crate::domain::fields::service::{names, type_of};
use crate::domain::names::service::base_of;
use crate::domain::nodes::types::Node;
use crate::domain::signatures::service::listing;
use crate::domain::signatures::types::Listing;
use crate::domain::syntax::service::{ancestors, kind, kind_bits, parent};
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `inherits` relation to, which reads its rows from a file's tree.
pub(crate) struct InheritsItem;

/// The type declaration that holds the embedded field at `field`, or the struct or interface type that does.
fn declaration(tree: &languages::Tree<'_>, field: languages::Position) -> languages::Position {
    let list = parent(tree, field);
    ancestors(tree, field)
        .find(|each| kind(tree, *each) == Kind::Node(Node::TypeSpec))
        .unwrap_or_else(|| parent(tree, list))
}

ryo::reads! {
    InheritsItem => languages::Inherits |tree, at, symbols| {
        kinds: [Kind::Node(Node::Field)] as kind_bits,
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

/// The type declaration that holds the embedded field at `field` of a walked file, or the struct or interface type that does.
fn walked_declaration(tree: &crate::domain::file_view::types::Syntax<'_>, field: languages::Position) -> languages::Position {
    let list = walked_syntax::parent(tree, field);
    walked_syntax::ancestors(tree, field)
        .find(|each| walked_syntax::kind(tree, *each) == Kind::Node(Node::TypeSpec))
        .unwrap_or_else(|| walked_syntax::parent(tree, list))
}

// The engine fills `inherits` from the walked file: each type an interface or a struct embeds.
ryo::fills! {
    InheritsItem => languages::Inherits, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Inherits as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for &at in node_table::ids_of_kind(tree.table, kind_bits(Kind::Node(Node::Field))) {
            if !matches!(
                crate::domain::walked::signatures::listing(&tree, walked_syntax::parent(&tree, at)),
                Some(Listing::Members | Listing::Methods)
            ) {
                continue;
            }
            if crate::domain::walked::fields::names(&tree, at).next().is_some() {
                continue;
            }
            let Some(base) = crate::domain::walked::fields::type_of(&tree, at) else { continue };
            let Some(target) = crate::domain::walked::names::base_of(&tree, &words, base) else { continue };
            rows.write(Row {
                occurrence: walked_syntax::node_cell(&tree, base),
                declaration: walked_syntax::node_cell(&tree, walked_declaration(&tree, at)),
                target,
                file: tree.file,
                span: walked_syntax::span_of(&tree, base),
            });
        }
    }
}
