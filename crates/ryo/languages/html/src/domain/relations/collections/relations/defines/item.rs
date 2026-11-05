//! The `defines` relation, read from the HTML DOM: a sectioning element titled by a heading is a section, named by its title and held
//! by the section around it; an `id`, or the `name` of an anchor element, is a label.
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{defined, kind_bits, kind_symbol, owner, section_name, value};
use crate::domain::syntax::types::{Defined, Kind};
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

#[cfg(test)]
#[path = "item_test.rs"]
mod tests;
