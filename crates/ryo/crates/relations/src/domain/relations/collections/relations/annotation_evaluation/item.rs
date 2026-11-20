//! For each element inside a Python annotation, when the language evaluates that annotation.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId};
use crate::domain::terms::types::Symbol;
/// For each element inside an annotation, the annotation and when the language evaluates it.
#[derive(Clone, Copy, Debug)]
pub struct AnnotationEvaluation;
impl super::super::interface::sealed::Sealed for AnnotationEvaluation {}
/// Declare the reusable static annotation-timing fact.
#[macro_export]
#[doc(hidden)]
macro_rules! annotation_evaluation {
    ($callback:path) => {
        $callback! {
        item AnnotationEvaluation,
        row { /// One element of one annotation.
        AnnotationEvaluationRow },
        column { /// Public fact columns.
        AnnotationEvaluationColumn },
        expected { /// Independently specified annotation timing.
        AnnotationEvaluationExpectedRow },
        class Code,
        scope File,
        notes Unneeded,
        columns {
        /// An element directly inside the annotation: not in a lambda body, a generator's element or later clauses, or an
        /// embedded expression, which have their own evaluation.
        id Id: NodeId => Node;
        /// The annotation expression the element belongs to.
        root Root: NodeId => Node;
        /// eager, postponed, deferred or unevaluated.
        evaluation Evaluation: Symbol => Symbol;
        /// lazy where the file stores annotations without evaluating them, eager otherwise.
        mode Mode: Symbol => Symbol;
        /// Source file defining these identities.
        file File: FileId => File;
        }
        }
    };
}
crate::annotation_evaluation!(crate::relation_row);
