//! One collection: the interface every relation implements, and each relation in order.

pub mod analyzed_source;
pub mod annotates;
pub mod assigns;
pub mod call_argument;
pub mod comment;
pub mod decorates;
pub mod defines;
pub mod directive;
pub mod entry;
pub mod file_package;
pub mod filesystem_file;
pub mod filesystem_line_count;
pub mod format_field;
pub mod format_parse;
pub mod function;
pub mod inherits;
pub mod interface;
pub mod mapping;
pub mod mention;
pub mod package_dependency;
pub mod parameter;
pub mod prose;
pub mod reference;
pub mod resolves;
pub mod scope;
pub mod sequence_element;
pub mod source_dependency;
pub mod source_location;
pub mod static_call;
pub mod static_import;
pub mod string_literal;
pub mod syntax_child;
pub mod syntax_error;
pub mod syntax_tree;
pub mod visibility;
pub mod workspace_member;

pub mod indent_region;

/// Canonical lexical name provenance.
pub mod qualified_name;

/// Canonical lexical identifier occurrences.
pub mod identifier;

pub mod semantic_syntax;

pub mod semantic_child;

pub mod embedded_expression;

/// Optional static binding analysis.
pub mod binding_event;
/// Structural expression identity.
pub mod expression_shape;
/// Decoded literal fragments of interpolated strings.
pub mod interpolated_string_literal;
pub mod language_target;
/// Typed primitive literal properties.
pub mod literal_properties;

/// Canonical keys for constant expressions.
pub mod constant_key;

/// Optional static binding analysis.
pub mod binding_read;

/// Optional static binding analysis.
pub mod binding_use;

/// Optional static binding analysis.
pub mod binding_shadow;

/// Optional occurrence-independent language builtin catalog.
pub mod implicit_symbol;
