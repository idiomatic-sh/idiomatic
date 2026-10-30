//! One collection: the interface every relation implements, and each relation of the language in order.

pub(crate) mod annotates;
pub(crate) mod assigns;
pub(crate) mod call_argument;
pub(crate) mod comment;
pub(crate) mod decorates;
pub(crate) mod defines;
pub(crate) mod function;
pub(crate) mod inherits;
pub(crate) mod mention;
pub(crate) mod parameter;
pub(crate) mod reference;
pub(crate) mod resolves;
pub(crate) mod scope;
pub(crate) mod static_call;
pub(crate) mod static_import;
pub(crate) mod string_literal;
pub(crate) mod syntax_child;
pub(crate) mod syntax_error;
pub(crate) mod syntax_tree;
pub(crate) mod visibility;

#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
