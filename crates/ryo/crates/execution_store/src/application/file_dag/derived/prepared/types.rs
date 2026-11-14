//! The types of the `prepared` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::application::file_dag::derived::constants as activation;
use crate::application::file_dag::derived::owner::cache::types::GraphAdmission;
use ryo_execution_domain::domain::{
    coverage::types::RuleIndex,
    operating::errors::OperatingError,
};
use ryo_relations::domain::relations::types::Relation;

/// What one rule of a request needs of the derived views: the relations it reads per file, each once in the order first read, and
/// whether it walks a graph.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::derived::owner::cache::types::GraphAdmission;
/// use ryo_execution_store::application::file_dag::derived::prepared::types::RuleDerivedNeeds;
/// use ryo_relations::domain::relations::types::Relation;
/// let needs = RuleDerivedNeeds { relations: Box::new([Relation::Function]), graph: GraphAdmission::Disabled };
/// assert_eq!(needs.relations.len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleDerivedNeeds {
    /// The relations the rule reads per file.
    pub relations: Box<[Relation]>,
    /// Whether the rule walks a graph, so may request graph admission.
    pub graph: GraphAdmission,
}
