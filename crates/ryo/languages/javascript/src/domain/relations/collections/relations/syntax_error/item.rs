//! JavaScript `syntax_error` facts from the `ESTree` projection.
use crate::domain::nodes::types::JavaScriptNode;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;
/// Reads the `syntax-error` rows of one JavaScript file from its syntax tree.
pub(crate) struct SyntaxErrorItem;
ryo::reads! {SyntaxErrorItem => languages::SyntaxError |tree,at,symbols|{kinds:[Kind::Node(JavaScriptNode::Program)] as kind_bits,
each (languages::diagnostics(tree.elements)) as (number, diagnostic) {
            given phase = languages::intern_derived(symbols, tree.source, languages::NameText("parse"));
            given impact = languages::intern_derived(symbols, tree.source, languages::NameText("incomplete"));
            given code = languages::intern_derived(symbols, tree.source, languages::NameText("unspecified"));
row{id:value(languages::diagnostic_id(tree.file, number)),message:value(languages::intern_derived(symbols,tree.source,languages::NameText(std::str::from_utf8(diagnostic.message.0).unwrap_or_default()))),phase:value(phase),impact:value(impact),code:value(code),file:file,span:value(languages::Span{start:diagnostic.start,end:diagnostic.end})}}}}
