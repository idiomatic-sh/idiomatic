//! The `reference` relation of the language: a name used where it is not declared, and a dotted name used, by the whole dotted text:
//! `a.b.c` is one reference, not also `a`. The names a declaration writes are no reference, and neither is the name after a dot or a
//! label. The key of a composite literal is a field or a variable, which the language cannot tell apart without types: its target is the
//! unknown value of an ambiguous key.

use crate::domain::names::service::{dotted, unknown};
use crate::domain::names::types::Unresolved;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::owner;
use crate::domain::slots::service::referencing;
use crate::domain::slots::types::Referencing;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `reference` relation to, which reads its rows from a file's tree.
pub(crate) struct ReferenceItem;

ryo::reads! {
    ReferenceItem => languages::Reference |tree, at, symbols| {
        kinds: [Kind::Node(Node::Ident), Kind::Node(Node::SelectorExpr)] as kind_bits,
        when referencing(tree, at) {
            Referencing::Used => {
                given target = dotted(tree, symbols, at)?;
                row {
                    occurrence: node(at),
                    owner: value(owner(tree, at)),
                    target: value(target),
                    file: file,
                    span: span(at),
                }
            }
            Referencing::Unclear => {
                row {
                    occurrence: node(at),
                    owner: value(owner(tree, at)),
                    target: value(unknown(Unresolved::AmbiguousKey)),
                    file: file,
                    span: span(at),
                }
            }
            Referencing::Other => {}
        }
    }
}

// The engine fills `reference` from the walked file: a name used where it is not declared, and a dotted name used, by the whole dotted text.
ryo::fills! {
    ReferenceItem => languages::Reference, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Reference as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for wanted in [Kind::Node(Node::Ident), Kind::Node(Node::SelectorExpr)] {
            for &at in node_table::ids_of_kind(tree.table, kind_bits(wanted)) {
                let target = match crate::domain::walked::slots::referencing(&tree, at) {
                    Referencing::Used => {
                        let Some(target) = crate::domain::walked::names::dotted(&tree, &words, at) else { continue };
                        target
                    }
                    Referencing::Unclear => crate::domain::walked::names::unknown(Unresolved::AmbiguousKey),
                    Referencing::Other => continue,
                };
                rows.write(Row {
                    occurrence: walked_syntax::node_cell(&tree, at),
                    owner: crate::domain::walked::scopes::owner(&tree, at),
                    target,
                    file: tree.file,
                    span: walked_syntax::span_of(&tree, at),
                });
            }
        }
    }
}
