//! TypeScript `string_literal` rows from its standard syntax tree: string literals, and template literals with no substitution,
//! each with the value the parser decoded (a template's cooked value).
use crate::domain::fact_notes::service::specifier;
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service as scopes;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `string-literal` rows of one TypeScript file from its syntax tree.
pub(crate) struct StringLiteralItem;

// The engine fills `string-literal` from the walked file: every row of the file, once.
ryo::fills! {
    StringLiteralItem => languages::StringLiteral, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StringLiteral as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::Literal), Kind::Node(Node::TemplateLiteral)]) {
            let Some(value) = names::string(file, &mut words, at) else {
                continue;
            };
            rows.write(Row {
                occurrence: node_cell(file, at),
                owner: scopes::owner(file, at),
                value,
                file: file.file,
                span: span_at(file, at),
                import: specifier(file, at),
            });
        }
    }
}
