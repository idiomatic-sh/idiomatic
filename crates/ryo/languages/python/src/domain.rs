//! The python domain: its nodes, tokens and extras, its files, its scopes and its names, and the readers of its relations.

pub(crate) mod bindings;
pub(crate) mod definitions;
pub(crate) mod dependencies;
pub(crate) mod extras;
pub(crate) mod fact_notes;
pub(crate) mod fields;
pub(crate) mod imports;
pub(crate) mod labels;
pub(crate) mod language;
pub(crate) mod links;
pub(crate) mod manifests;
pub(crate) mod merging;
pub(crate) mod model;
pub(crate) mod names;
pub(crate) mod nodes;
pub(crate) mod packages;
pub(crate) mod patterns;
pub(crate) mod relations;
pub(crate) mod scopes;
pub(crate) mod siblings;
pub(crate) mod syntax;
pub(crate) mod tokens;

pub(crate) mod indentation;

/// Lexical import and builtin provenance.
pub(crate) mod qualification;

pub(crate) mod semantic_syntax;

/// Sparse environments used by optional static binding analysis.
pub(crate) mod binding_states;
/// Hashable parser-supported constant expressions.
pub(crate) mod constant_keys;
/// Structural expression identity.
pub(crate) mod expression_shapes;
/// Parsed primitive categories and narrow value predicates.
pub(crate) mod literal_properties;
pub(crate) mod numeric_literals;

/// Ordered language events and explicitly deferred evaluation phases.
pub(crate) mod binding_schedule;

/// Static binding observations with explicit absence and uncertainty.
pub(crate) mod binding_results;

/// Static lookup preserves branch absence and real candidate identities.
pub(crate) mod binding_lookup;

/// Static event observations preserve real source identities.
pub(crate) mod binding_observations;

/// Lazy execution of static binding phases and alternatives.
pub(crate) mod binding_analysis;

/// Explicit uncertainty for repeated and exceptional namespace effects.
pub(crate) mod binding_uncertainty;
/// Lazy brace-format parsing, separate from percent-format parsing.
pub(crate) mod brace_format;
/// Basic types inferred from individual expressions.
pub(crate) mod expression_types;
/// Lazy per-source percent-format parse results.
pub(crate) mod format_parsing;
/// Application-owned decoded format grammar results and parser port.
pub(crate) mod format_syntax;
pub(crate) mod raw_format;
/// Terminal outcomes of function bodies.
pub(crate) mod terminal_flow;

/// Lazy syntactic validity of original quoted expression carriers.
pub(crate) mod literal_expression_parsing;
/// Project-established importable module context.
pub(crate) mod module_context;

/// Bounded application-owned grammar reuse.
pub(crate) mod grammar_cache;
