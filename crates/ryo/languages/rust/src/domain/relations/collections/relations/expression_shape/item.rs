//! The `expression_shape` relation of the language: each node under the file's root and the class of nodes structurally equal to
//! it, from the file's shapes. The root is the file, which no other node equals.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::shapes::service::prepare;
use crate::domain::syntax::constants::SHAPES_INCOMPLETE;
use crate::domain::syntax::service::{is_token, kind_bits};
use crate::domain::syntax::types::{Kind, Leaf};
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `expression_shape` relation's rows to; a tree the parser could not read completely
/// gives none.
pub(crate) struct ExpressionShapeItem;

ryo::reads! {
    ExpressionShapeItem => languages::ExpressionShape, model(ReadIndexes) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::SourceFile)] as kind_bits,
        when prepare(tree, model, symbols) {
            Some(index) => {
                each (languages::positions(tree.elements)) as expression {
                    require is_token(tree, expression) == Leaf::Node && expression != at;
                    given shape = languages::shape_class_at(&index, expression)?;
                    row { id: node(expression), shape: node(shape), file: file, span: span(expression) }
                }
            }
            None => {
                partial SHAPES_INCOMPLETE => languages::Missing::Any;
            }
        }
    }
}
