//! TypeScript `resolves` rows: each type reference to a single identifier, with the declaration, import or type parameter the parser's
//! semantic pass binds the name to where it is written.
use crate::domain::bindings::service as bindings;
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::nodes::types::Node;
use crate::domain::resolutions::service as resolutions;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `resolves` rows of one TypeScript file from its syntax tree.
pub(crate) struct ResolvesItem;
ryo::reads! { ResolvesItem => languages::Resolves, model(ReadIndexes) |tree, model, at, symbols| {
 kinds: [Kind::Node(Node::TSTypeReference)] as kind_bits,
 given written = resolutions::plain_type_name(tree, at)?;
 given name = names::identifier(tree, symbols, written);
 given index = bindings::indexed(tree, &model.bindings);
 given definition = bindings::declaration(tree, &index, written)?;
 row { occurrence: node(at), owner: value(resolutions::holder(tree, at)), name: value(name), definition: node(definition), file: file, span: span(at) }
} }
