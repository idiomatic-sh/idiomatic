//! The `string_literal` relation of the language: every string, f-string and t-string, its parts joined as the library decoded them;
//! one that interpolates has the unknown value of an interpolated string. A bytes `Constant` is no string, and the format spec of an
//! interpolation is part of its f-string. The name `__import__(..)` or `importlib.import_module(..)` loads is an import specifier.



use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_facts::service::{specifier as loaded, string_value};
use crate::domain::literal_facts::types::StringValue;

use crate::domain::model::view::service::built;
use crate::domain::model::view::types::ModelPart;

use crate::domain::names::service::unknown;
use crate::domain::names::types::Unresolved;
use crate::domain::node::service::{node_cell, semantic_nodes_of_kind, span_at, word};
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Node;



use crate::domain::syntax::types::Kind;

use ryo::languages;

/// Reads the `string-literal` rows of one Python file from its syntax tree.
pub(crate) struct StringLiteralItem;

// The engine fills `string-literal` from the walked file: every row of the file, once.
ryo::fills! {
    StringLiteralItem => languages::StringLiteral, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StringLiteral as languages::CatalogRelation>::Row;
        // The owner of a node is the scope the model says it is evaluated in.
        built(file.model, &mut file.built, ModelPart::Lexical, file.syntax.tree);
        let owned = &file.model.lexical.scope_of;
        let tree = file.syntax.tree;
        let joined = &mut file.buffers.text;
        for kind in [Node::Constant, Node::JoinedStr, Node::TemplateStr] {
            for Numbered { id, node } in semantic_nodes_of_kind(&file.syntax, Kind::Node(kind)) {
                let value = match string_value(node, joined) {
                    Some(StringValue::Known(text)) => word(words, text),
                    Some(StringValue::Interpolated) => unknown(Unresolved::InterpolatedString),
                    None => continue,
                };
                rows.write(Row {
                    occurrence: node_cell(&file.syntax, id),
                    owner: node_cell(&file.syntax, crate::domain::node::service::place_of_id(tree, id).and_then(|p| owned.get(usize::from(p))).map_or(languages::Position(0), |&p| crate::domain::node::service::table_id(tree.segments, p))),
                    value,
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                    import: loaded(&file.syntax, id),
                });
            }
        }
    }
}
