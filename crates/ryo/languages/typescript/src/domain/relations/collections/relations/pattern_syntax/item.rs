//! TypeScript `pattern_syntax` rows: every element of the regular-expression patterns the file carries.
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
    PatternSyntaxItem => languages::PatternSyntax |tree, at, symbols| {
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
