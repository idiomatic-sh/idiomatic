//! Parsed annotation expressions retain their host, context, and physical mapping.
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
