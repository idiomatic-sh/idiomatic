//! Parsed annotation expressions retain their host, context, and physical mapping.
use crate::domain::file_view::types::PythonWalk;
use crate::domain::node::service::{fixed, table_id};
use crate::domain::semantic_syntax::constants::{DEFERRED_CONTEXT, EXACT_MAPPING, HOST_MAPPING};
use crate::domain::semantic_syntax::types::Visibility;
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;
/// Reads each enabled annotation expression once, from the native module root. It reads what completion adds, so it is
/// `expanded`: a request that wants it has the annotations expanded. Expanded elements hold no module, so it still reads once.
pub(crate) struct EmbeddedExpressionItem;
ryo::reads! {
    EmbeddedExpressionItem => languages::EmbeddedExpression, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        each (languages::embedded_expressions(tree.elements)) as embedded {
            require crate::domain::semantic_syntax::service::visibility(tree, model, embedded.root, symbols) == Visibility::Visible;
            given context = languages::intern_derived(symbols, tree.source, languages::NameText(DEFERRED_CONTEXT));
            given mapping = languages::intern_derived(symbols, tree.source, languages::NameText(match embedded.mapping {
                languages::EmbeddedMapping::Exact(_) => EXACT_MAPPING,
                languages::EmbeddedMapping::WholeHost => HOST_MAPPING,
            }));
            row { root: node(embedded.root), host: node(embedded.host), context: value(context), mapping: value(mapping), file: file, span: span(embedded.root), }
        }
    }
}

// The engine fills `embedded-expression` from the walked file: every row of the file, once. The walk embeds the expression of a
// string only where the language reads the string as a type expression, so every segment it wrote is a row.
ryo::fills! {
    EmbeddedExpressionItem => languages::EmbeddedExpression, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::EmbeddedExpression as languages::CatalogRelation>::Row;
        let memo = &mut file.buffers.fixed;
        let segments = file.syntax.tree.segments;
        for segment in segments {
            let span = languages::node_table::span_of(file.syntax.table, segment.root);
            rows.write(Row {
                root: languages::node_id(file.syntax.file, segment.root),
                host: languages::node_id(file.syntax.file, table_id(segments, segment.host)),
                context: fixed(words, memo, languages::NameText(DEFERRED_CONTEXT)),
                mapping: fixed(words, memo, languages::NameText(match segment.mapping {
                    languages::EmbeddedMapping::Exact(_) => EXACT_MAPPING,
                    languages::EmbeddedMapping::WholeHost => HOST_MAPPING,
                })),
                file: file.syntax.file,
                span: languages::Span { start: span.start, end: span.end },
            });
        }
    }
}
