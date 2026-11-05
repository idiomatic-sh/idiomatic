//! The `visibility` relation of the language: the visibility an item, a field or a variant is written with, as written.

use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{collapsed, kind_bits};
use crate::domain::syntax::types::Kind;
use crate::domain::visibilities::service::visible;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `visibility` relation's rows to.
pub(crate) struct VisibilityItem;

ryo::reads! {
    VisibilityItem => languages::Visibility |tree, at, symbols| {
        kinds: [Kind::Node(Node::Visibility)] as kind_bits,
        each (visible(tree, at)) as declaration {
            row {
                occurrence: node(at),
                declaration: node(declaration),
                text: value(collapsed(tree, symbols, at)),
                file: file,
                span: span(at),
            }
        }
    }
}
