//! The parser's diagnostics, including malformed UTF-8 and recovered syntax.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::{INCOMPLETE_IMPACT, PARSE_PHASE, UNSPECIFIED_CODE};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `syntax-error` rows of one TypeScript file from its syntax tree.
pub(crate) struct SyntaxErrorItem;

ryo::reads! {
    SyntaxErrorItem => languages::SyntaxError, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Program)] as kind_bits,
        each (languages::diagnostics(tree.elements)) as (number, diagnostic) {
            given phase = languages::intern_derived(symbols, tree.source, languages::NameText(PARSE_PHASE));
            given impact = languages::intern_derived(symbols, tree.source, languages::NameText(INCOMPLETE_IMPACT));
            given code = languages::intern_derived(symbols, tree.source, languages::NameText(UNSPECIFIED_CODE));
            row {
                id: value(languages::diagnostic_id(tree.file, number)),
                message: value(languages::intern_derived(symbols, tree.source, languages::NameText(std::str::from_utf8(diagnostic.message.0).unwrap_or_default()))),
                phase: value(phase), impact: value(impact), code: value(code), file: file,
                span: value(languages::Span { start: diagnostic.start, end: diagnostic.end }),
            }
        }
    }
}
