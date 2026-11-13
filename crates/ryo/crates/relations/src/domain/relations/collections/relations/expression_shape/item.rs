//! Collision-safe classes of structurally equal parsed expressions.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
/// One expression and its opaque file-local structural class.
#[derive(Clone, Copy, Debug)]
pub struct ExpressionShape;
impl super::super::interface::sealed::Sealed for ExpressionShape {}
/// Declare structural identity without exposing hashes or parser-library types.
#[macro_export]
#[doc(hidden)]
macro_rules! expression_shape {
    ($callback:path) => {
        $callback! {
         item ExpressionShape,
         row { /// One known expression class.
          ExpressionShapeRow },
         column { /// Structural identity columns.
          ExpressionShapeColumn },
         expected { /// Independently specified structural identity.
          ExpressionShapeExpectedRow },
         class Code,
         scope File,
         notes Unneeded,
         columns {
          /// The original expression occurrence.
          id Id: NodeId=>Node;
          /// An opaque real syntax representative in the same file. Compare with file; never interpret it as a hash or source order.
          shape Shape: NodeId=>Node;
          /// The source file defining this class namespace.
          file File: FileId=>File;
          /// The original expression extent; normalization never rewrites source positions.
          span Span: Span=>Span;
         }
        }
    };
}
crate::expression_shape!(crate::relation_row);
