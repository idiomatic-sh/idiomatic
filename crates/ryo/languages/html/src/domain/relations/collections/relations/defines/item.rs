//! The `defines` relation, read from the HTML DOM: a sectioning element titled by a heading is a section, named by its title and held
//! by the section around it; an `id`, or the `name` of an anchor element, is a label.
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::{Defined, Kind};
use crate::domain::file_view::types::HtmlWalk;
use crate::domain::file_view::service::{elements, kind as view_kind, node_cell, property, span_at, word};
use crate::domain::walked_values::service as walked;
use crate::domain::syntax::service::kind_name;
use crate::domain::syntax::types::Property;
use ryo::languages;
/// The item the engine's fill of the `defines` relation is registered on.
pub(crate) struct DefinesItem;

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

