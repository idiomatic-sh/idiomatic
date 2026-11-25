//! TypeScript `pattern_child` rows: the edges of the regular-expression patterns the file carries, in pattern order.
use crate::domain::file_view::service::{node_cell, of_kinds};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::syntax::reading::Syntax as _;
use crate::domain::model::types::ReadIndexes;
use crate::domain::patterns::service as patterns;
use crate::domain::syntax::types::PatternNode;
use crate::domain::syntax::{
    service::{kind_bits, parent},
    types::Kind,
};
use ryo::languages;

/// Reads the `pattern-child` rows of one TypeScript file from its syntax tree.
pub(crate) struct PatternChildItem;
ryo::reads! {
    PatternChildItem => languages::PatternChild, model(ReadIndexes) |tree, _model, at, symbols| {
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
        require (patterns::nested(tree, at) == languages::Flag::Raised);
        row {
            parent: node(parent(tree, at)),
            child: node(at),
            ordinal: value(languages::Integer(i64::from(languages::ordinal_at(tree.elements, at).0))),
            field: value(patterns::field(tree, symbols, at)),
            file: file,
        }
    }
}

// The engine fills `pattern-child` from the walked file: every element of a pattern under another of its own.
ryo::fills! {
    PatternChildItem => languages::PatternChild, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::PatternChild as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        for at in of_kinds(file, KINDS) {
            if patterns::nested(file, at) != languages::Flag::Raised {
                continue;
            }
            rows.write(Row {
                parent: node_cell(file, parent(file, at)),
                child: node_cell(file, at),
                ordinal: languages::Integer(i64::from(file.ordinal_of(at).0)),
                field: patterns::field(file, &mut words, at),
                file: file.file,
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
