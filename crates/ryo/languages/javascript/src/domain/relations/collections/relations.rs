//! One collection: each relation the language reads from a file's tree, in order.

pub(crate) mod syntax_tree;

pub(crate) mod call_argument;

#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;

pub(crate) mod comment;

pub(crate) mod string_literal;

pub(crate) mod function;

pub(crate) mod parameter;

pub(crate) mod static_call;

pub(crate) mod static_import;

pub(crate) mod syntax_child;

pub(crate) mod syntax_error;

pub(crate) mod assigns;
pub(crate) mod defines;
pub(crate) mod directive;
pub(crate) mod inherits;
pub(crate) mod reference;
pub(crate) mod scope;
pub(crate) mod visibility;
