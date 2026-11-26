//! The `defines` relation of the language: a heading is a section, the definition of a link reference or of a footnote is a label, and
//! so is an anchor raw HTML gives. A section is named by its title, and held by the section around it.

use crate::domain::file_view::service::{node_cell, sectioned, span_at, word};
use crate::domain::file_view::types::MarkdownWalk;
use crate::domain::labels::service::anchors_in;
use crate::domain::labels::types::Defined;
use crate::domain::syntax::service::kind_name;
use crate::domain::walked_model::service::{defined as walked_defined, owner as walked_owner};
use crate::domain::walked_prose::service::section_text;
use ryo::languages::node_table;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The item the engine's fill of the relation is registered on.
pub(crate) struct DefinesItem;

// The engine fills `defines` from the walked file: each heading a section, each definition and footnote a label, each anchor of raw HTML.
ryo::fills! {
    DefinesItem => languages::Defines, walk(MarkdownWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Defines as languages::CatalogRelation>::Row;
        let (sections, syntax, text) = sectioned(file);
        for held in [Node::Heading, Node::FootnoteDefinition, Node::Definition, Node::Html] {
            for &id in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Node(held))) {
                let refinement = word(words, kind_name(Kind::Node(held)).0);
                match walked_defined(&syntax, id) {
                    Defined::Section => {
                        section_text(&syntax, id, text);
                        rows.write(Row {
                            id: node_cell(&syntax, id),
                            kind: languages::DeclarationKind::Section,
                            refinement,
                            name: word(words, text),
                            owner: walked_owner(&syntax, sections, id),
                            file: syntax.file,
                            span: span_at(&syntax, id),
                        });
                    }
                    Defined::Label(label) => rows.write(Row {
                        id: node_cell(&syntax, id),
                        kind: languages::DeclarationKind::Label,
                        refinement,
                        name: word(words, label.0),
                        owner: walked_owner(&syntax, sections, id),
                        file: syntax.file,
                        span: span_at(&syntax, id),
                    }),
                    Defined::Html(html) => {
                        for anchor in anchors_in(html) {
                            rows.write(Row {
                                id: node_cell(&syntax, id),
                                kind: languages::DeclarationKind::Label,
                                refinement,
                                name: word(words, anchor.0),
                                owner: walked_owner(&syntax, sections, id),
                                file: syntax.file,
                                span: span_at(&syntax, id),
                            });
                        }
                    }
                    Defined::Nothing => {}
                }
            }
        }
    }
}
