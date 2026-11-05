//! The types of the `rules` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::query_bindings::types::ParameterBinding;
use crate::domain::files::types::Glob;
use crate::domain::plan_parts::types::ParameterName;
use crate::domain::program::types::QueryBundle;
use crate::domain::runs::types::{RuleApplicability, RuleId};

/// A rule's parameters as they are written, for the digest of the configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleParameters(pub Box<str>);

/// One rule of a run, as the domain reads it: its id, its query, what its parameters are bound to and the globs of the files it governs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleSpec {
    /// The rule's id, named in every finding.
    pub id: RuleId,
    /// The RyoQL text.
    pub query: QueryBundle,
    /// Whether this rule is evaluated only after the provider accepts syntax.
    pub applicability: RuleApplicability,
    /// What the rule binds its parameters to, by name.
    pub given: Vec<Given>,
    /// Explicit named producer inputs, separate from externally hosted static rows.
    pub observations: Vec<crate::domain::observations::types::ObservationBinding>,
    /// The parameters as they are written, for the digest of the configuration.
    pub spelled: RuleParameters,
    /// The globs of the files the rule governs; none means every file.
    pub include: Vec<Glob>,
    /// The globs of the files it does not govern.
    pub exclude: Vec<Glob>,
}

/// What one parameter of a rule is bound to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Given {
    /// The parameter's name.
    pub name: ParameterName,
    /// Its value.
    pub value: ParameterBinding,
}
