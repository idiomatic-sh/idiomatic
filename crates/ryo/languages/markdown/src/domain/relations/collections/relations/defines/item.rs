//! The `defines` relation of the language: a heading is a section, the definition of a link reference or of a footnote is a label, and
//! so is an anchor raw HTML gives. A section is named by its title, and held by the section around it.

use crate::domain::labels::service::{anchors_in, defined};
use crate::domain::labels::types::Defined;
use crate::domain::model::service::owner;
use crate::domain::model::types::Sections;
use crate::domain::nodes::types::Node;
use crate::domain::prose::service::section_name;
use crate::domain::syntax::service::{kind_bits, kind_symbol, name_of};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `defines` relation to, which reads its rows from a file's tree.
pub(crate) struct DefinesItem;

ryo::reads! {
    DefinesItem => languages::Defines, model(Sections) |tree, model, at, symbols| {
        kinds: [
            Kind::Node(Node::Heading),
            Kind::Node(Node::FootnoteDefinition),
            Kind::Node(Node::Definition),
            Kind::Node(Node::Html),
        ] as kind_bits,
        when defined(tree, at) {
            Defined::Section => {
                row {
                    id: node(at),
                    kind: value(languages::DeclarationKind::Section),
                    refinement: value(kind_symbol(tree, symbols, at)),
                    name: value(section_name(tree, symbols, at)),
                    owner: value(owner(tree, model, at)),
                    file: file,
                    span: span(at),
                }
            }
            Defined::Label(label) => {
                row {
                    id: node(at),
                    kind: value(languages::DeclarationKind::Label),
                    refinement: value(kind_symbol(tree, symbols, at)),
                    name: value(name_of(tree, symbols, languages::NameText(label.0))),
                    owner: value(owner(tree, model, at)),
                    file: file,
                    span: span(at),
                }
            }
            Defined::Html(html) => {
                each (anchors_in(html)) as anchor {
                    row {
                        id: node(at),
                        kind: value(languages::DeclarationKind::Label),
                        refinement: value(kind_symbol(tree, symbols, at)),
                        name: value(name_of(tree, symbols, languages::NameText(anchor.0))),
                        owner: value(owner(tree, model, at)),
                        file: file,
                        span: span(at),
                    }
                }
            }
            Defined::Nothing => {}
        }
    }
}
