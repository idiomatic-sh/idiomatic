//! TypeScript `pattern_parse` rows: the outcome of each regular-expression pattern the file carries, as the language's library read it.
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::patterns::service as patterns;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `pattern-parse` rows of one TypeScript file from its syntax tree.
pub(crate) struct PatternParseItem;
ryo::reads! {
    PatternParseItem => languages::PatternParse, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Literal)] as kind_bits,
        given flags = patterns::flags(tree, symbols, at)?;
        row {
            carrier: node(at),
            flags: value(flags),
            outcome: value(patterns::outcome(tree, symbols, at)),
            error: value(patterns::error(tree, symbols, at)),
            file: file,
            span: span(at),
        }
    }
}
