//! One collection: each relation the language reads from a file's tree, in order.

pub(crate) mod annotates;
pub(crate) mod assigns;
pub(crate) mod comment;
pub(crate) mod decorates;
pub(crate) mod defines;
pub(crate) mod function;
pub(crate) mod inherits;
pub(crate) mod parameter;
pub(crate) mod reference;
pub(crate) mod scope;
pub(crate) mod static_call;
pub(crate) mod static_import;
pub(crate) mod string_literal;
pub(crate) mod syntax_child;
pub(crate) mod syntax_error;
pub(crate) mod syntax_tree;

pub(crate) mod call_argument;

pub(crate) mod resolves;

#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;

pub(crate) mod indent_region;

/// Canonical lexical name provenance.
pub(crate) mod qualified_name;

/// Canonical lexical identifiers.
pub(crate) mod identifier;

pub(crate) mod semantic_syntax;

pub(crate) mod semantic_child;

pub(crate) mod embedded_expression;

/// Generic static binding analysis.
pub(crate) mod binding_event;
/// Structural expression identity.
pub(crate) mod expression_shape;

/// Generic static binding analysis.
pub(crate) mod binding_read;

/// Generic static binding analysis.
pub(crate) mod binding_use;

/// Generic static binding analysis.
pub(crate) mod binding_shadow;

/// Optional builtin inventory independent of Name expressions.
pub(crate) mod implicit_symbol;
