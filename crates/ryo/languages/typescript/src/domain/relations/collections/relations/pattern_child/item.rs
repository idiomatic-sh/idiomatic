//! TypeScript `pattern_child` rows: the edges of the regular-expression patterns the file carries, in pattern order.
use crate::domain::patterns::service as patterns;
use crate::domain::syntax::types::PatternNode;
use crate::domain::syntax::{
    service::{kind_bits, parent},
    types::Kind,
};
use crate::domain::model::types::ReadIndexes;
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
