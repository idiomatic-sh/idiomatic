//! Parsed annotation expressions retain their host, context, and physical mapping.
use crate::domain::file_view::types::PythonWalk;
use crate::domain::model::names::service::fixed_word;
use crate::domain::node::service::table_id;
use crate::domain::semantic_syntax::constants::{DEFERRED_CONTEXT, EXACT_MAPPING, HOST_MAPPING};


use ryo::languages;
/// Reads each enabled annotation expression once, from the native module root. It reads what completion adds, so it is
/// `expanded`: a request that wants it has the annotations expanded. Expanded elements hold no module, so it still reads once.
pub(crate) struct EmbeddedExpressionItem;

// The engine fills `embedded-expression` from the walked file: every row of the file, once. The walk embeds the expression of a
// string only where the language reads the string as a type expression, so every segment it wrote is a row.
ryo::fills! {
    EmbeddedExpressionItem => languages::EmbeddedExpression, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::EmbeddedExpression as languages::CatalogRelation>::Row;
        let memo = &mut file.model.words.get_mut().fixed;
        let segments = file.syntax.tree.segments;
        for segment in segments {
            let span = languages::node_table::span_of(file.syntax.table, segment.root);
            rows.write(Row {
                root: languages::node_id(file.syntax.file, segment.root),
                host: languages::node_id(file.syntax.file, table_id(segments, segment.host)),
                context: fixed_word(words, memo, languages::NameText(DEFERRED_CONTEXT)),
                mapping: fixed_word(words, memo, languages::NameText(match segment.mapping {
                    languages::EmbeddedMapping::Exact(_) => EXACT_MAPPING,
                    languages::EmbeddedMapping::WholeHost => HOST_MAPPING,
                })),
                file: file.syntax.file,
                span: languages::Span { start: span.start, end: span.end },
            });
        }
    }
}
