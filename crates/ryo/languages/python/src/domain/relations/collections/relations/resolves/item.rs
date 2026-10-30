//! The `resolves` relation of the language: each name written in an annotation, with the class, import or type alias that binds it
//! where it is written. A name qualified by a dot is not resolved.

use crate::domain::definitions::service::{binding_of, holder, type_names};
use crate::domain::model::types::Declarations;
use crate::domain::names::service::annotation_of;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, name_of};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `resolves` rows of one Python file from its syntax tree.
pub(crate) struct ResolvesItem;

ryo::reads! {
    ResolvesItem => languages::Resolves, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Arg), Kind::Node(Node::FunctionDef), Kind::Node(Node::AnnAssign)] as kind_bits,
        given annotated = annotation_of(tree, at)?;
        each (type_names(tree, annotated.annotation)) as written {
            given name = name_of(tree, written)?;
            given definition = binding_of(tree, model, at, name)?;
            row {
                occurrence: node(written),
                owner: value(holder(tree, at)),
                name: value(name),
                definition: node(definition),
                file: file,
                span: span(written),
            }
        }
    }
}
