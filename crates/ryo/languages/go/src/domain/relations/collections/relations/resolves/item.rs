//! The `resolves` relation of the language: each identifier written as a type, with the `type` declaration of the file or the block
//! that binds it where it is written. A name qualified by a package (`pkg.Type`) and a type of another file of the package are not
//! resolved.

use crate::domain::names::service::name_of;
use crate::domain::nodes::types::GoNode;
use crate::domain::specs::service::{binding_of, holder, written_as_type};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `resolves` relation to, which reads its rows from a file's tree.
pub(crate) struct ResolvesItem;

ryo::reads! {
    ResolvesItem => languages::Resolves |tree, at, symbols| {
        kinds: [Kind::Node(GoNode::Ident)] as kind_bits,
        given typed = written_as_type(tree, at)?;
        given name = name_of(tree, typed)?;
        given definition = binding_of(tree, at, name)?;
        row {
            occurrence: node(at),
            owner: value(holder(tree, at)),
            name: value(name),
            definition: node(definition),
            file: file,
            span: span(at),
        }
    }
}
