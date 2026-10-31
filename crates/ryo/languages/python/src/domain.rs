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

/// Structural expression identity.
pub(crate) mod expression_shapes;
/// Sparse environments used by optional static binding analysis.
pub(crate) mod binding_states;

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
