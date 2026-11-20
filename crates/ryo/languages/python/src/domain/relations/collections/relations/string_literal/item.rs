//! The `string_literal` relation of the language: every string, f-string and t-string, its parts joined as the library decoded them;
//! one that interpolates has the unknown value of an interpolated string. A bytes `Constant` is no string, and the format spec of an
//! interpolation is part of its f-string. The name `__import__(..)` or `importlib.import_module(..)` loads is an import specifier.
use crate::domain::semantic_syntax::types::Visibility;

use crate::domain::fact_notes::service::specifier;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_facts::service::{specifier as loaded, string_value};
use crate::domain::literal_facts::types::{StringValue, TextBuffer};
use crate::domain::model::types::Declarations;
use crate::domain::names::service::string_or_unknown;
use crate::domain::names::service::unknown;
use crate::domain::names::types::Unresolved;
use crate::domain::node::service::{node_cell, nodes_of_kind, span_at, word};
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Node;
use crate::domain::owner_facts::service::{owner_of, owners};
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::service::{kind, literal, standing};
use crate::domain::syntax::types::Kind;
use crate::domain::syntax::types::{Literal, Standing};
use ryo::languages;

/// Reads the `string-literal` rows of one Python file from its syntax tree.
pub(crate) struct StringLiteralItem;

ryo::reads! {
    StringLiteralItem => languages::StringLiteral, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Constant), Kind::Node(Node::JoinedStr), Kind::Node(Node::TemplateStr)] as kind_bits, expanded,
        require crate::domain::semantic_syntax::service::visibility(tree, model, at, symbols) == Visibility::Visible;
        require literal(tree, at) == Literal::String || kind(tree, at) != Kind::Node(Node::Constant);
        require standing(tree, at) == Standing::Own;
        row {
            occurrence: node(at),
            owner: value(owner(tree, at)),
            value: value(string_or_unknown(tree, symbols, at)),
            file: file,
            span: span(at),
            import: value(specifier(tree, at)),
        }
    }
}

// The engine fills `string-literal` from the walked file: every row of the file, once.
ryo::fills! {
    StringLiteralItem => languages::StringLiteral, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StringLiteral as languages::CatalogRelation>::Row;
        let owned = owners(&file.syntax, file.owners);
        let mut joined = TextBuffer::default();
        for kind in [Node::Constant, Node::JoinedStr, Node::TemplateStr] {
            for Numbered { id, node } in nodes_of_kind(&file.syntax, Kind::Node(kind)) {
                let value = match string_value(node, &mut joined) {
                    Some(StringValue::Known(text)) => word(words, text),
                    Some(StringValue::Interpolated) => unknown(Unresolved::InterpolatedString),
                    None => continue,
                };
                rows.write(Row {
                    occurrence: node_cell(&file.syntax, id),
                    owner: node_cell(&file.syntax, owner_of(owned, id)),
                    value,
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                    import: loaded(&file.syntax, id),
                });
            }
        }
    }
}
