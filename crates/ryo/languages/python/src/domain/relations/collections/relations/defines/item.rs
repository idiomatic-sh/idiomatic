//! The `defines` relation of the language: every name a file binds: functions and methods, classes, imported names, assigned names,
//! parameters, the names of `except ... as` and of `case` captures, type aliases and type parameters, each in the scope that binds it
//! once `global` and `nonlocal` are read. `from m import *` defines names it cannot list: its name is the unknown value of a
//! wildcard import.

use crate::domain::definitions::service::{definition, word};
use crate::domain::model::types::Declarations;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `defines` rows of one Python file from its syntax tree.
pub(crate) struct DefinesItem;

ryo::reads! {
    DefinesItem => languages::Defines, model(Declarations) |tree, model, at, symbols| {
        kinds: [
            Kind::Node(Node::FunctionDef),
            Kind::Node(Node::ClassDef),
            Kind::Node(Node::Alias),
            Kind::Node(Node::Name),
            Kind::Node(Node::Arg),
            Kind::Node(Node::ExceptHandler),
            Kind::Node(Node::MatchAs),
            Kind::Node(Node::MatchStar),
            Kind::Node(Node::MatchMapping),
            Kind::Node(Node::TypeVar),
            Kind::Node(Node::TypeVarTuple),
            Kind::Node(Node::ParamSpec),
        ] as kind_bits,
        given defined = definition(tree, model, at)?;
        row {
            id: node(at),
            kind: value(defined.defined.kind),
            refinement: value(word(tree, symbols, defined.defined.word)),
            name: value(defined.name),
            owner: node(defined.owner),
            file: file,
            span: span(defined.named),
        }
    }
}
