//! The `static_import` relation of the language: each name a `use` brings into scope, by the path it imports.

use crate::domain::imports::service::{import_of, leaves};
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::{child, kind_bits};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `static_import` relation's rows to.
pub(crate) struct StaticImportItem;

ryo::reads! {
    StaticImportItem => languages::StaticImport, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Use)] as kind_bits,
        given use_tree = child(tree, at, Kind::Node(Node::UseTree))?;
        each (leaves(tree, use_tree)) as leaf {
            given import = import_of(tree, symbols, leaf)?;
            row {
                occurrence: node(leaf),
                owner: value(owner(tree, at)),
                binding: value(import.binding),
                target: value(import.target),
                file: file,
                span: span(leaf),
            }
        }
    }
}
