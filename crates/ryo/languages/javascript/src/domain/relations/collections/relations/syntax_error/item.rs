//! JavaScript `syntax_error` facts from the `ESTree` projection.
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;
/// Reads the `syntax-error` rows of one JavaScript file from its syntax tree.
pub(crate) struct SyntaxErrorItem;
ryo::reads! {SyntaxErrorItem => languages::SyntaxError |tree,at,symbols|{kinds:[Kind::Node(Node::Program)] as kind_bits,
each (languages::diagnostics(tree.elements)) as (number,diagnostic) {
row{id:value(languages::diagnostic_id(tree.file, number)),message:value(languages::intern_derived(symbols,tree.source,languages::NameText(std::str::from_utf8(diagnostic.message.0).unwrap_or_default()))),file:file,span:value(languages::Span{start:diagnostic.start,end:diagnostic.end})}}}}
