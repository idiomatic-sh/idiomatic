//! The `reference` relation of the language: every link and every image, with what it points at, resolved to a path of the run as an
//! import is (`[a](../guide.md#setup)` in `docs/a.md` points at `guide.md#setup`).

use crate::domain::model::service::owner;
use crate::domain::model::types::Sections;
use crate::domain::nodes::types::MarkdownNode;
use crate::domain::syntax::service::{kind_bits, name_of, named_by};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `reference` relation to, which reads its rows from a file's tree.
pub(crate) struct ReferenceItem;

ryo::reads! {
    ReferenceItem => languages::Reference, model(Sections) |tree, model, at, symbols| {
        kinds: [
            Kind::Node(MarkdownNode::Link),
            Kind::Node(MarkdownNode::LinkReference),
            Kind::Node(MarkdownNode::Image),
            Kind::Node(MarkdownNode::ImageReference),
        ] as kind_bits,
        given destination = named_by(tree, at)?;
        row {
            occurrence: node(at),
            owner: value(owner(tree, model, at)),
            target: value(name_of(tree, symbols, destination)),
            file: file,
            span: span(at),
        }
    }
}
