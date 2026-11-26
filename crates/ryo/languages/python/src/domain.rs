//! The python domain: its nodes, tokens and extras, its files, its scopes and its names, and the readers of its relations.

pub(crate) mod bindings;
pub(crate) mod definitions;
pub(crate) mod dependencies;
pub(crate) mod expression_fields;
pub(crate) mod extras;
pub(crate) mod fact_notes;
pub(crate) mod fields;
pub(crate) mod file_view;
pub(crate) mod imports;
pub(crate) mod keyed_rows;
pub(crate) mod labels;
pub(crate) mod language;
pub(crate) mod layout_facts;
pub(crate) mod links;
pub(crate) mod manifests;
pub(crate) mod merging;
pub(crate) mod model;
pub(crate) mod name_facts;
pub(crate) mod names;
pub(crate) mod node;
pub(crate) mod node_fields;
pub(crate) mod nodes;
pub(crate) mod packages;
pub(crate) mod patterns;
pub(crate) mod relations;
pub(crate) mod rendering;
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
pub(crate) mod constant_numbers;
/// Structural expression identity.
pub(crate) mod expression_shapes;
/// Parsed primitive categories and narrow value predicates.
pub(crate) mod literal_facts;
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
pub(crate) mod annotation_evaluation;
pub(crate) mod binding_analysis;

/// Spike (ryo-fmx3): schedule-order name resolution over the binding analysis.
pub(crate) mod binding_resolution;

/// The lookup path of each read or deletion: the namespaces its name is looked up in.
pub(crate) mod lookup_path;

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
/// Buffers the readers keep from file to file instead of allocating per file.
pub(crate) mod read_scratch;
/// Target-aware recognition of Python's standard format-specification mini-language.
pub(crate) mod standard_format_spec;
/// Python release-specific, platform-independent standard-library names.
pub(crate) mod standard_library;
/// Terminal outcomes of function bodies.
pub(crate) mod terminal_flow;

/// Lazy syntactic validity of original quoted expression carriers.
pub(crate) mod literal_expression_parsing;
/// Project-established importable module context.
pub(crate) mod module_context;
pub(crate) mod project_namespace;

/// Bounded application-owned grammar reuse.
pub(crate) mod grammar_cache;

/// Standard-format source observations without lint call policy.
pub(crate) mod format_specification;

/// Physical native-token successor and delimiter edges.
pub(crate) mod token_links;

/// Standard project-metadata validation outcomes.
pub(crate) mod manifest_validation;

/// The rows of the lexical relations, written from the model.
pub(crate) mod lexical_facts;

/// The rows of the binding relations, written from the model.
pub(crate) mod binding_facts;

/// How the body of a function ends, from the parser's own statements.
pub(crate) mod terminal_facts;

/// The basic type the syntax of an expression gives it, from the parser's own nodes.
pub(crate) mod type_facts;

/// Which constants are equal as Python values, from the parser's own nodes.
pub(crate) mod key_facts;

/// Which expressions are written with the same structure, from the parser's own nodes.
pub(crate) mod shape_facts;

/// The format specifications a file writes, from the parser's own nodes.
pub(crate) mod spec_facts;

/// Which strings the language reads as type expressions, from the parser's own nodes and the model's resolution.
pub(crate) mod string_annotations;
