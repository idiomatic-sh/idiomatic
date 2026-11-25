//! TypeScript `pattern_syntax` rows: every element of the regular-expression patterns the file carries.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::model::types::ReadIndexes;
use crate::domain::patterns::service as patterns;
use crate::domain::syntax::types::PatternNode;
use crate::domain::syntax::{
    service::{kind_bits, kind_symbol},
    types::Kind,
};
use ryo::languages;

/// Reads the `pattern-syntax` rows of one TypeScript file from its syntax tree.
pub(crate) struct PatternSyntaxItem;
ryo::reads! {
    PatternSyntaxItem => languages::PatternSyntax, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [
  Kind::Pattern(PatternNode::Pattern),
  Kind::Pattern(PatternNode::Disjunction),
  Kind::Pattern(PatternNode::Alternative),
  Kind::Pattern(PatternNode::BoundaryAssertion),
  Kind::Pattern(PatternNode::LookAroundAssertion),
  Kind::Pattern(PatternNode::Quantifier),
  Kind::Pattern(PatternNode::Character),
  Kind::Pattern(PatternNode::Dot),
  Kind::Pattern(PatternNode::CharacterClassEscape),
  Kind::Pattern(PatternNode::UnicodePropertyEscape),
  Kind::Pattern(PatternNode::CharacterClass),
  Kind::Pattern(PatternNode::CharacterClassRange),
  Kind::Pattern(PatternNode::ClassStringDisjunction),
  Kind::Pattern(PatternNode::ClassString),
  Kind::Pattern(PatternNode::CapturingGroup),
  Kind::Pattern(PatternNode::IgnoreGroup),
  Kind::Pattern(PatternNode::IndexedReference),
  Kind::Pattern(PatternNode::NamedReference),
        ] as kind_bits, expanded,
        row {
            id: node(at),
            carrier: node(patterns::carrier(tree, at)),
            kind: value(kind_symbol(tree, symbols, at)),
            text: value(patterns::text(tree, symbols, at)),
            value: value(patterns::value(tree, at)),
            file: file,
            span: span(at),
        }
    }
}

// The engine fills `pattern-syntax` from the walked file: every element of every pattern written under a carrier.
ryo::fills! {
    PatternSyntaxItem => languages::PatternSyntax, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::PatternSyntax as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        for at in of_kinds(file, KINDS) {
            rows.write(Row {
                id: node_cell(file, at),
                carrier: node_cell(file, patterns::carrier(file, at)),
                kind: kind_symbol(file, &mut words, at),
                text: patterns::text(file, &mut words, at),
                value: patterns::value(file, at),
                file: file.file,
                span: span_at(file, at),
            });
        }
    }
}

/// The kinds of the elements of a pattern.
const KINDS: &[Kind] = &[
            Kind::Pattern(PatternNode::Pattern), Kind::Pattern(PatternNode::Disjunction), Kind::Pattern(PatternNode::Alternative),
            Kind::Pattern(PatternNode::BoundaryAssertion), Kind::Pattern(PatternNode::LookAroundAssertion), Kind::Pattern(PatternNode::Quantifier),
            Kind::Pattern(PatternNode::Character), Kind::Pattern(PatternNode::Dot), Kind::Pattern(PatternNode::CharacterClassEscape),
            Kind::Pattern(PatternNode::UnicodePropertyEscape), Kind::Pattern(PatternNode::CharacterClass), Kind::Pattern(PatternNode::CharacterClassRange),
            Kind::Pattern(PatternNode::ClassStringDisjunction), Kind::Pattern(PatternNode::ClassString), Kind::Pattern(PatternNode::CapturingGroup),
            Kind::Pattern(PatternNode::IgnoreGroup), Kind::Pattern(PatternNode::IndexedReference), Kind::Pattern(PatternNode::NamedReference),
        ];
