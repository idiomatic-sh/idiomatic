//! The `prose` relation of the language: the text a reader reads, in runs: a paragraph, the title of a heading, a cell of a table, and
//! the text an item of a list holds itself. Its markup is joined, its code, math and raw HTML are masked.

use crate::domain::model::service::owner;
use crate::domain::model::types::Sections;
use crate::domain::nodes::types::Node;
use crate::domain::prose::service::{has_words, run_name};
use crate::domain::prose::types::Wording;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `prose` relation to, which reads its rows from a file's tree.
pub(crate) struct ProseItem;

ryo::reads! {
    ProseItem => languages::Prose, model(Sections) |tree, model, at, symbols| {
        kinds: [
            Kind::Node(Node::Paragraph),
            Kind::Node(Node::Heading),
            Kind::Node(Node::ListItem),
            Kind::Node(Node::TableCell),
        ] as kind_bits,
        require (has_words(tree, at) == Wording::Words);
        row {
            id: node(at),
            owner: value(owner(tree, model, at)),
            text: value(run_name(tree, symbols, at)),
            file: file,
            span: span(at),
        }
    }
}
