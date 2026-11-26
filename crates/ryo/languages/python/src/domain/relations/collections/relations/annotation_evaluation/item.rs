//! Write when Python evaluates each element of an annotation, once from the module root of the old route and once from the walk.

use super::constants::INCOMPLETE;
use crate::domain::annotation_evaluation::{
    service::{Old, Shape as _, rows, target},
    types::Row,
};
use crate::domain::file_view::types::PythonWalk;
use crate::domain::model::names::service::fixed_word;
use crate::domain::relations::collections::relations::compiler_feature::item::feature_flag;
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    syntax::{constants::MODULE_FUTURE_ANNOTATIONS, service::kind_bits, types::Kind},
};
use ryo::languages::{self, Position};

/// Reads when each element of an annotation is evaluated.
pub(crate) struct AnnotationEvaluationItem;

/// Whether the old route's tree was read whole.
fn syntax_state(tree: &languages::Tree<'_>) -> languages::SyntaxCompleteness {
    languages::has_incomplete_syntax(tree.elements)
}

thread_local! {
    /// The rows of the old route's last file, kept between files so a file allocates nothing after the first.
    static OLD_ROWS: std::cell::RefCell<Vec<Row>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Reads the rows of the old route's table into the kept buffer, when it is whole and holds an annotation; how many.
fn evaluated(tree: &languages::Tree<'_>, model: &Declarations, at: Position) -> usize {
    OLD_ROWS.with(|held| {
        let mut held = held.borrow_mut();
        held.clear();
        let old = Old(tree);
        if old.has_annotation() {
            let future = feature_flag(languages::property_at(tree.elements, at, MODULE_FUTURE_ANNOTATIONS))
                .map(|flag| flag == languages::Flag::Raised);
            let target = target(future, model.language_target.major.0, model.language_target.minor.0);
            rows(&old, &target, languages::embedded_expressions(tree.elements).map(|source| source.root), &mut |row| held.push(row));
        }
        held.len()
    })
}

/// The row of the old route's last file at `index`.
fn row_at(index: usize) -> Row {
    OLD_ROWS.with(|held| held.borrow()[index])
}

ryo::reads! {
    AnnotationEvaluationItem => languages::AnnotationEvaluation, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        when syntax_state(tree) {
            languages::SyntaxCompleteness::Incomplete => { partial INCOMPLETE => languages::Missing::Any; }
            languages::SyntaxCompleteness::Complete => {
                each (0..evaluated(tree, model, at)) as index {
                    given evaluation = languages::intern_derived(symbols, tree.source, languages::NameText(row_at(index).evaluation));
                    given mode = languages::intern_derived(symbols, tree.source, languages::NameText(row_at(index).mode));
                    row { id: node(row_at(index).id), root: node(row_at(index).root), evaluation: value(evaluation), mode: value(mode), file: file, }
                }
            }
        }
    }
}

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
