//! The `parameter` relation of the language: a parameter of a function, a method or a closure.

use crate::domain::bindings::service::{binding_of, parameters_of, spread};
use crate::domain::bindings::types::Spread;
use crate::domain::nodes::types::Node;
use crate::domain::refinements::service::{positional_name, receiver_name, variadic_name};
use crate::domain::relations::constants::SELF_VALUE;
use crate::domain::scopes::service::is_nameless_function;
use crate::domain::scopes::types::Namelessness;
use crate::domain::syntax::service::{child, interned, kind, kind_bits};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `parameter` relation's rows to.
pub(crate) struct ParameterItem;

ryo::reads! {
    ParameterItem => languages::Parameter |tree, at, symbols| {
        kinds: [Kind::Node(Node::Fn), Kind::Node(Node::ClosureExpr)] as kind_bits,
        require is_nameless_function(tree, at) == Namelessness::Named;
        given list = child(tree, at, Kind::Node(Node::ParamList))?;
        each (parameters_of(tree, list)) as (ordinal, parameter) {
            when kind(tree, parameter) {
                Kind::Node(Node::SelfParam) => {
                    row {
                        id: node(parameter),
                        kind: value(languages::ParameterKind::Receiver),
                        refinement: value(interned(tree, symbols, receiver_name())),
                        name: value(interned(tree, symbols, languages::NameText(SELF_VALUE))),
                        ordinal: value(languages::Integer(i64::from(ordinal.0))),
                        owner: node(at),
                        file: file,
                        span: span(parameter),
                    }
                }
                _ => {
                    given bound = binding_of(tree, parameter);
                    when spread(tree, parameter) {
                        Spread::Rest => {
                            row {
                                id: node(bound.id),
                                kind: value(languages::ParameterKind::VariadicPositional),
                                refinement: value(interned(tree, symbols, variadic_name())),
                                name: value(bound.name),
                                ordinal: value(languages::Integer(i64::from(ordinal.0))),
                                owner: node(at),
                                file: file,
                                span: span(parameter),
                            }
                        }
                        Spread::One => {
                            row {
                                id: node(bound.id),
                                kind: value(languages::ParameterKind::Positional),
                                refinement: value(interned(tree, symbols, positional_name())),
                                name: value(bound.name),
                                ordinal: value(languages::Integer(i64::from(ordinal.0))),
                                owner: node(at),
                                file: file,
                                span: span(parameter),
                            }
                        }
                    }
                }
            }
        }
    }
}
