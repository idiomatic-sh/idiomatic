//! The `defines` relation, read from the HTML DOM: a sectioning element titled by a heading is a section, named by its title and held
//! by the section around it; an `id`, or the `name` of an anchor element, is a label.
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{defined, kind_bits, kind_symbol, owner, section_name, value};
use crate::domain::syntax::types::{Defined, Kind};
use crate::domain::file_view::types::HtmlWalk;
use crate::domain::file_view::service::{elements, kind as view_kind, node_cell, property, span_at, word};
use crate::domain::walked_values::service as walked;
use crate::domain::syntax::service::kind_name;
use crate::domain::syntax::types::Property;
use ryo::languages;
/// The reader the `parse` contract hands the `defines` relation to, which reads its rows from a file's tree.
pub(crate) struct DefinesItem;
ryo::reads! {
    DefinesItem => languages::Defines |tree, at, symbols| {
        kinds: [Kind::Node(Node::Element), Kind::Node(Node::Attribute)] as kind_bits,
        when defined(tree, at) {
            Defined::Section(heading) => {
                row {
                    id: node(at),
                    kind: value(languages::DeclarationKind::Section),
                    refinement: value(kind_symbol(tree, symbols, at)),
                    name: value(section_name(tree, symbols, heading)),
                    owner: node(owner(tree, at)),
                    file: file,
                    span: span(at),
                }
            }
            Defined::Label => {
                row {
                    id: node(at),
                    kind: value(languages::DeclarationKind::Label),
                    refinement: value(kind_symbol(tree, symbols, at)),
                    name: value(value(tree, symbols, at)),
                    owner: node(owner(tree, at)),
                    file: file,
                    span: span(at),
                }
            }
            Defined::Nothing => {}
        }
    }
}

// The engine fills `defines` from the walked file: each titled section, named by its title, and each label.
ryo::fills! {
    DefinesItem => languages::Defines, walk(HtmlWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Defines as languages::CatalogRelation>::Row;
        let view = *file;
        for at in elements(&view) {
            if !matches!(view_kind(&view, at), Kind::Node(Node::Element | Node::Attribute)) {
                continue;
            }
            let (declared, name) = match walked::defined(&view, at) {
                Defined::Section(heading) => (languages::DeclarationKind::Section, word(words, &walked::section_name(&view, heading))),
                Defined::Label => (languages::DeclarationKind::Label, word(words, property(&view, at, Property::Value))),
                Defined::Nothing => continue,
            };
            let refinement = word(words, kind_name(view_kind(&view, at)).0);
            rows.write(Row {
                id: node_cell(&view, at),
                kind: declared,
                refinement,
                name,
                owner: node_cell(&view, walked::owner(&view, at)),
                file: view.file,
                span: span_at(&view, at),
            });
        }
    }
}

#[cfg(test)]
#[path = "item_test.rs"]
mod tests;
