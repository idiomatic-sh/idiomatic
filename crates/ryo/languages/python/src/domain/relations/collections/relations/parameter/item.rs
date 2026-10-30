//! The `parameter` relation of the language: each parameter of a function or a lambda, in its place, by how it takes its argument;
//! the markers `/` and `*` are no parameters.

use crate::domain::definitions::service::{placed, word};
use crate::domain::extras::types::Extra;
use crate::domain::model::types::Declarations;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{child, kind_bits, name_of};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `parameter` rows of one Python file from its syntax tree.
pub(crate) struct ParameterItem;

ryo::reads! {
    ParameterItem => languages::Parameter, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Arg)] as kind_bits,
        given place = placed(tree, at)?;
        given identifier = child(tree, at, Kind::Extra(Extra::Identifier))?;
        given name = name_of(tree, identifier)?;
        row {
            id: node(at),
            kind: value(place.kind),
            refinement: value(word(tree, symbols, place.word)),
            name: value(name),
            ordinal: value(place.ordinal),
            owner: node(place.callable),
            file: file,
            span: span(at),
        }
    }
}
