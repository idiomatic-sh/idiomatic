//! The types of the `rules` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::table_model::types::ParameterBinding;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::plan_parts::types::{ParameterName, QueryText};
use ryo_values::domain::runs::types::RuleId;

/// A rule's parameters as they are written, for the digest of the configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleParameters(pub Box<str>);

/// One rule of a run, as the domain reads it: its id, its query, what its parameters are bound to and the globs of the files it governs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleSpec {
    /// The rule's id, named in every finding.
    pub id: RuleId,
    /// The RyoQL text.
    pub query: QueryText,
    /// What the rule binds its parameters to, by name.
    pub given: Vec<Given>,
    /// The parameters as they are written, for the digest of the configuration.
    pub spelled: RuleParameters,
    /// The globs of the files the rule governs; none means every file.
    pub include: Vec<Glob>,
    /// The globs of the files it does not govern.
    pub exclude: Vec<Glob>,
}

/// What one parameter of a rule is bound to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Given {
    /// The parameter's name.
    pub name: ParameterName,
    /// Its value.
    pub value: ParameterBinding,
}
