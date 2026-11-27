//! Shared vocabulary for when a language evaluates the expression of an annotation.
use super::item::AnnotationEvaluationColumn;
/// Canonical query relation name.
#[must_use]
pub fn name() -> &'static str {
    "annotation-evaluation"
}
/// Meaning of an annotation evaluation row.
#[must_use]
pub(crate) fn summary() -> &'static str {
    "When, if ever, each element inside an annotation is evaluated: with its definition, kept as text, evaluated when first asked, or never. A language whose annotations are erased answers never; one that has no annotations declares the relation absent."
}
/// Public name of each typed column.
#[must_use]
pub(crate) fn column_name(column: AnnotationEvaluationColumn) -> &'static str {
    match column {
        AnnotationEvaluationColumn::Id => "id",
        AnnotationEvaluationColumn::Root => "root",
        AnnotationEvaluationColumn::Evaluation => "evaluation",
        AnnotationEvaluationColumn::Mode => "mode",
        AnnotationEvaluationColumn::File => "file",
    }
}
