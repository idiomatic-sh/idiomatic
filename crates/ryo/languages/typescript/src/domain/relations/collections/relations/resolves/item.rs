//! TypeScript `resolves` rows: each type reference to a single identifier, with the declaration, import or type parameter the parser's
//! semantic pass binds the name to where it is written.
use crate::domain::bindings::service as bindings;
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::resolutions::service as resolutions;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `resolves` rows of one TypeScript file from its syntax tree.
pub(crate) struct ResolvesItem;

// The engine fills `resolves` from the walked file: every row of the file, once.
ryo::fills! {
    ResolvesItem => languages::Resolves, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Resolves as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::TSTypeReference)]) {
            let Some(written) = resolutions::plain_type_name(file, at) else {
                continue;
            };
            let name = names::identifier(file, &mut words, written);
            let index = bindings::indexed(file, &file.model.bindings);
            let Some(definition) = bindings::declaration(file, &index, written) else {
                continue;
            };
            rows.write(Row {
                occurrence: node_cell(file, at),
                owner: resolutions::holder(file, at),
                name,
                definition: node_cell(file, definition),
                file: file.file,
                span: span_at(file, at),
            });
        }
    }
}
