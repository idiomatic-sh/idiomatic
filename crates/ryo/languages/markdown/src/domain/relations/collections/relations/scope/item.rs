//! The `scope` relation of the language: the document, which encloses itself, and a section under the section that holds it, the
//! document's for the outermost, as far as the next heading as shallow or shallower.

use crate::domain::model::service::scoped;
use crate::domain::model::types::{Scoped, Sections};
use crate::domain::nodes::types::MarkdownNode;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `scope` relation to, which reads its rows from a file's tree.
pub(crate) struct ScopeItem;

ryo::reads! {
    ScopeItem => languages::Scope, model(Sections) |tree, model, at, _symbols| {
        kinds: [
            Kind::Node(MarkdownNode::Root),
            Kind::Node(MarkdownNode::Heading),
        ] as kind_bits,
        when scoped(tree, model, at) {
            Scoped::Document => {
                row { parent: node(at), child: node(at), file: file, span: span(at) }
            }
            Scoped::Section(section) => {
                row {
                    parent: value(languages::node_id(tree.file, section.parent)),
                    child: node(at),
                    file: file,
                    span: value(languages::Span {
                        start: languages::start_at(tree.elements, at),
                        end: section.end,
                    }),
                }
            }
            Scoped::Nothing => {}
        }
    }
}
