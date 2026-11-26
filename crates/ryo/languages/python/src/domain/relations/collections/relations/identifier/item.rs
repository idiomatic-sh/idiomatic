//! Emit parser-normalized spelling without changing lossless source tokens.
use crate::domain::extras::types::Extra as TypedExtra;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::node::service::{node_cell, nodes_of_kind, span_at, table_id, word};
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Node as TypedNode;

use crate::domain::syntax::constants::IDENTIFIERS_INCOMPLETE as INCOMPLETE_IDENTIFIERS;
use crate::domain::syntax::types::Kind as TypedKind;

use ruff_python_ast::AnyNodeRef;
use ryo::languages;
/// Python lexical identifier occurrences.
pub(crate) struct IdentifierItem;

// The engine fills `identifier` from the walked file: every row of the file, once.
ryo::fills! {
    IdentifierItem => languages::Identifier, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Identifier as languages::CatalogRelation>::Row;
        if file.syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE_IDENTIFIERS), languages::Missing::Any);
            return;
        }
        for kind in [TypedKind::Node(TypedNode::Name), TypedKind::Extra(TypedExtra::Identifier)] {
            for Numbered { id, node } in nodes_of_kind(&file.syntax, kind) {
                // An identifier with no name is the parser's stand-in for a missing one, and `*` is no name.
                let name = if let AnyNodeRef::ExprName(name) = node {
                    name.id.as_str()
                } else if let AnyNodeRef::Identifier(identifier) = node {
                    identifier.id.as_str()
                } else {
                    continue;
                };
                if name.is_empty() || name == crate::domain::name_facts::constants::EVERYTHING {
                    continue;
                }
                rows.write(Row {
                    id: node_cell(&file.syntax, id),
                    name: word(words, languages::NameText(name)),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            }
        }
        // SPIKE (ryo-fmx3.1, D): the names of the embedded segments (the expression of a string read as a type), which the
        // table's list by kind does not hold: each by its place after the file's own nodes.
        let tree = file.syntax.tree;
        let first = tree.segments.first().map_or(tree.nodes.len(), |segment| usize::from(segment.first));
        for (place, node) in tree.nodes.iter().enumerate().skip(first) {
            let name = if let AnyNodeRef::ExprName(name) = node {
                name.id.as_str()
            } else if let AnyNodeRef::Identifier(identifier) = node {
                identifier.id.as_str()
            } else {
                continue;
            };
            if name.is_empty() || name == crate::domain::name_facts::constants::EVERYTHING {
                continue;
            }
            let id = table_id(tree.segments, languages::Position(u32::try_from(place).unwrap_or(u32::MAX)));
            rows.write(Row {
                id: node_cell(&file.syntax, id),
                name: word(words, languages::NameText(name)),
                file: file.syntax.file,
                span: span_at(&file.syntax, id),
            });
        }
    }
}
