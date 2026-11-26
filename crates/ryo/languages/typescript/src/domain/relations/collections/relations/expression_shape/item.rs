//! TypeScript `expression_shape` rows: each node under the file's root and the class of nodes structurally equal to it, from the
//! file's shapes. The root is the file, which no other node equals.
use crate::domain::file_view::service::{node_cell, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::shapes::service::Numbered;
use crate::domain::syntax::reading::{Syntax as _, WordSink};
use crate::domain::syntax::constants::SHAPES_INCOMPLETE;
use ryo::languages::{self, node_table};

/// Reads the `expression-shape` rows of one TypeScript file; a tree the parser could not read completely gives none.
pub(crate) struct ExpressionShapeItem;

// The engine fills `expression-shape` from the walked file: every row of the file, or none and the part it could not read.
ryo::fills! {
    ExpressionShapeItem => languages::ExpressionShape, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::ExpressionShape as languages::CatalogRelation>::Row;
        let file = &*file;
        if node_table::syntax_error_count(file.table).0 != 0 {
            rows.partial(languages::ReportText(SHAPES_INCOMPLETE), languages::Missing::Any);
            return;
        }
        let mut words = WordSink::new(words);
        let mut index = languages::RetainedKeyIndex::<languages::Position, languages::ShapeWord>::default();
        let count = node_table::element_count(file.table).0;
        index.classes.resize(count, None);
        let mut whole = true;
        for raw in (0..count).rev() {
            let at = languages::Position(u32::try_from(raw).unwrap_or(u32::MAX));
            if file.is_leaf(at) {
                continue;
            }
            index.scratch.clear();
            let mut sink = Numbered { index: &mut index, whole: true };
            let found = crate::domain::shapes::service::words(file, at, &mut words, &mut sink);
            if found.is_none() || !sink.whole {
                whole = false;
                continue;
            }
            let class = languages::intern_key(&mut index, at);
            if let Some(slot) = index.classes.get_mut(raw) {
                *slot = Some(class);
            }
        }
        if !whole {
            rows.partial(languages::ReportText(SHAPES_INCOMPLETE), languages::Missing::Any);
            return;
        }
        let root = languages::Position(0);
        for raw in 0..count {
            let expression = languages::Position(u32::try_from(raw).unwrap_or(u32::MAX));
            if file.is_leaf(expression) || expression == root {
                continue;
            }
            let Some(shape) = index.classes.get(raw).copied().flatten() else {
                continue;
            };
            rows.write(Row {
                id: node_cell(file, expression),
                shape: node_cell(file, shape),
                file: file.file,
                span: span_at(file, expression),
            });
        }
    }
}
