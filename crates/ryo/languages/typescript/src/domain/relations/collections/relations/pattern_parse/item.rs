//! TypeScript `pattern_parse` rows: the outcome of each regular-expression pattern the file carries, as the language's library read it.
use crate::domain::file_view::service::{node_cell, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::patterns::constants::{MALFORMED, MALFORMED_FLAGS, PARSED};
use crate::domain::syntax::reading::{Interner, WordSink};
use crate::domain::syntax::service::kind;
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
            span: value(patterns::span(tree, at)),
        }
    }
}

// The engine fills `pattern-parse` from the walked file: what the library made of each pattern a carrier gives it.
ryo::fills! {
    PatternParseItem => languages::PatternParse, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::PatternParse as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        for fact in &file.patterns {
            if kind(file, fact.host) != Kind::Node(Node::Literal) {
                continue;
            }
            let outcome = match (&fact.refusal, fact.flags_refused) {
                (None, _) => PARSED,
                (Some(_), false) => MALFORMED,
                (Some(_), true) => MALFORMED_FLAGS,
            };
            let derived = |words: &mut WordSink<'_>, text: &str| Interner::derived(words, file.source, languages::NameText(text));
            rows.write(Row {
                carrier: node_cell(file, fact.host),
                flags: derived(&mut words, &fact.flags),
                outcome: derived(&mut words, outcome),
                error: fact.refusal.as_deref().map_or(languages::NO_SYMBOL, |message| derived(&mut words, message)),
                file: file.file,
                span: fact.place.map_or_else(
                    || span_at(file, fact.host),
                    |(start, end)| languages::Span { start, end },
                ),
            });
        }
    }
}
