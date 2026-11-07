//! The `decorates` relation of the language: each decorator of a function or a class, in its place, by the dotted name it writes or
//! calls and the arguments it passes; any other decorator has the unknown value of a dynamic decorator.

use crate::domain::extras::types::Extra;
use crate::domain::model::types::Declarations;
use crate::domain::names::service::{decorator_arguments, decorator_ordinal, decorator_target};
use crate::domain::syntax::service::{kind_bits, parent};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `decorates` rows of one Python file from its syntax tree.
pub(crate) struct DecoratesItem;

ryo::reads! {
    DecoratesItem => languages::Decorates, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Extra(Extra::Decorator)] as kind_bits,
        row {
            occurrence: node(at),
            declaration: node(parent(tree, at)),
            ordinal: value(decorator_ordinal(tree, at)),
            target: value(decorator_target(tree, symbols, at)),
            arguments: value(decorator_arguments(tree, symbols, at)),
            file: file,
            span: span(at),
        }
    }
}
