//! When a language evaluates the expression of an annotation, computed by a language provider.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId};
use crate::domain::terms::types::Symbol;
/// The optional `annotation-evaluation` relation.
#[derive(Clone, Copy, Debug)]
pub struct AnnotationEvaluation;
impl super::super::interface::sealed::Sealed for AnnotationEvaluation {}
/// Declare the shared row without interpreting the words a language answers with.
#[macro_export]
#[doc(hidden)]
macro_rules! annotation_evaluation {
    ($callback:path $(, $($extra:tt)*)?) => {
        $callback! {
            $( $($extra)* )?
            item AnnotationEvaluation,
            row { /// When one element of an annotation is evaluated.
                AnnotationEvaluationRow },
            column { /// Annotation evaluation column names.
                AnnotationEvaluationColumn },
            expected { /// An independently expected annotation evaluation row.
                AnnotationEvaluationExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            form Filled(Whole),
            size PerElement,
            keys [],
            columns {
                /// An element inside an annotation.
                id Id: NodeId => Node;
                /// The annotation the element belongs to.
                root Root: NodeId => Node;
                /// When the element is evaluated: `eager` (when its definition or statement runs), `postponed` (kept as text, not evaluated unless asked), `deferred` (evaluated when first asked) or `unevaluated` (never).
                evaluation Evaluation: Symbol => Symbol;
                /// Whether the language evaluates annotations lazily (`lazy`) or when their statement runs (`eager`).
                mode Mode: Symbol => Symbol;
                /// The physical source file.
                file File: FileId => File;
            }
        }
    };
}
crate::annotation_evaluation!(crate::relation_row);
