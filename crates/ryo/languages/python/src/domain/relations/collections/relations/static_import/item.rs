//! The `static_import` relation of the language: each name an `import` or a `from ... import` brings in, by the path it imports, the
//! dots of a relative import kept; `from m import *` imports the module `m`, its target, and binds names it cannot list: its binding is the
//! unknown value of a wildcard import.

use crate::domain::file_view::types::PythonWalk;
use crate::domain::imports::service::{import_binding, import_target};
use crate::domain::model::types::Declarations;
use crate::domain::model::view::service::built;
use crate::domain::model::view::types::ModelPart;
use crate::domain::name_facts::service::imported as imported_names;
use crate::domain::names::service::unknown;
use crate::domain::names::types::Unresolved;
use crate::domain::node::service::{node_cell, nodes_of_kind, span_at, word};
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `static-import` rows of one Python file from its syntax tree.
pub(crate) struct StaticImportItem;

ryo::reads! {
    StaticImportItem => languages::StaticImport, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Alias)] as kind_bits,
        given binding = import_binding(tree, at)?;
        given target = import_target(tree, symbols, at)?;
        row {
            occurrence: node(at),
            owner: value(owner(tree, at)),
            binding: value(binding),
            target: value(target),
            file: file,
            span: span(at),
        }
    }
}

// The engine fills `static-import` from the walked file: every row of the file, once.
ryo::fills! {
    StaticImportItem => languages::StaticImport, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StaticImport as languages::CatalogRelation>::Row;
        // The owner of a node is the scope the model says it is evaluated in.
        built(file.model, &mut file.built, ModelPart::Lexical, file.syntax.tree);
        let owned = &file.model.lexical.scope_of;
        let names = &mut file.buffers.text;
        for Numbered { id, .. } in nodes_of_kind(&file.syntax, Kind::Node(Node::Alias)) {
            if let Some(found) = imported_names(&file.syntax, id, names) {
                rows.write(Row {
                    occurrence: node_cell(&file.syntax, id),
                    owner: node_cell(&file.syntax, owned.get(usize::from(id)).copied().unwrap_or(languages::Position(0))),
                    binding: found
                        .binding
                        .map_or(unknown(Unresolved::WildcardImport), |name| word(words, name)),
                    target: word(words, found.target),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            }
        }
    }
}
