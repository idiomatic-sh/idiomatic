//! Write when Python evaluates each element of an annotation, from the walked file.

use super::constants::INCOMPLETE;
use crate::domain::annotation_evaluation::{
    service::{Shape as _, rows, target},
};
use crate::domain::file_view::types::PythonWalk;
use crate::domain::model::names::service::fixed_word;
use crate::domain::relations::collections::relations::compiler_feature::item::feature_flag;
use crate::domain::syntax::constants::MODULE_FUTURE_ANNOTATIONS;
use ryo::languages::{self, Position};

/// Reads when each element of an annotation is evaluated.
pub(crate) struct AnnotationEvaluationItem;

// The engine fills `annotation-evaluation` from the walked file: every row of the file, once.
ryo::fills! {
    AnnotationEvaluationItem => languages::AnnotationEvaluation, walk(PythonWalk) |file, words, rows_out| {
        /// One row of the relation.
        type Out = <languages::AnnotationEvaluation as languages::CatalogRelation>::Row;
        if file.syntax.completeness.syntax != languages::SyntaxCompleteness::Complete {
            rows_out.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        } else if file.syntax.table.has_annotation() {
            let future = feature_flag(languages::node_table::property_of(file.syntax.table, Position(0), MODULE_FUTURE_ANNOTATIONS))
                .map(|flag| flag == languages::Flag::Raised);
            let target = target(future, file.target.major.0, file.target.minor.0);
            let memo = &mut file.model.words.get_mut().fixed;
            let number = file.syntax.file;
            rows(&file.syntax.table, &target, file.syntax.tree.segments.iter().map(|segment| segment.root), &mut |answer| {
                rows_out.write(Out {
                    id: languages::node_id(number, answer.id),
                    root: languages::node_id(number, answer.root),
                    evaluation: fixed_word(words, memo, languages::NameText(answer.evaluation)),
                    mode: fixed_word(words, memo, languages::NameText(answer.mode)),
                    file: number,
                });
            });
        }
    }
}
