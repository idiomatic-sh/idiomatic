//! Query vocabulary for annotation-evaluation.
use super::item::AnnotationEvaluationColumn;
/// Query relation name.
#[must_use]
pub fn name() -> &'static str {
    "annotation-evaluation"
}
/// The static semantic contract, independent of lint policy.
#[must_use]
pub fn summary() -> &'static str {
    "For each element directly inside a function-signature or variable annotation, that annotation and when the language evaluates it: eager, postponed (the annotations future), deferred (native lazy annotations) or unevaluated."
}
/// Query column name.
#[must_use]
pub fn column_name(column: AnnotationEvaluationColumn) -> &'static str {
    match column {
        AnnotationEvaluationColumn::Id => "id",
        AnnotationEvaluationColumn::Root => "root",
        AnnotationEvaluationColumn::Evaluation => "evaluation",
        AnnotationEvaluationColumn::Mode => "mode",
        AnnotationEvaluationColumn::File => "file",
    }
}
