//! The `prepared` application operation.

use super::types::RuleDerivedNeeds;
use crate::application::file_dag::derived::constants as activation;
use crate::application::file_dag::derived::owner::cache::types::GraphAdmission;
use ryo_execution_domain::domain::{coverage::types::RuleIndex, operating::errors::OperatingError};
use ryo_relations::domain::relations::types::Relation;

/// The derived needs of a compiled request: the relations each rule reads and the rules that walk a graph.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::derived::prepared::service::RequestDerived;
/// fn named(_: Option<&RequestDerived>) {}
/// ```
#[derive(Debug)]
pub struct RequestDerived {
    relations: Box<[Box<[Relation]>]>,
    graph_rules: rustc_hash::FxHashSet<RuleIndex>,
}

impl RequestDerived {
    /// The derived needs of a request, one per rule in rule order.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::owner::cache::types::GraphAdmission;
    /// use ryo_execution_store::application::file_dag::derived::prepared::{service::RequestDerived, types::RuleDerivedNeeds};
    /// use ryo_relations::domain::relations::types::Relation;
    /// use ryo_execution_store::application::file_dag::fixtures::program_derived_port;
    /// use ryo_execution_store::application::file_dag::relations::{interface::RelationsPort, service::RelationOwner};
    /// use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
    /// let owner = RelationOwner::fixture(ReuseEligibility::Eligible, program_derived_port());
    /// let needs = RuleDerivedNeeds { relations: Box::new([Relation::Function]), graph: GraphAdmission::Disabled };
    /// let request = RequestDerived::new(Box::new([needs]));
    /// owner.derived().prepare_program_file(&request, &[], &[])?;
    /// # Ok::<(), ryo_execution_domain::domain::operating::errors::OperatingError>(())
    /// ```
    #[must_use]
    pub fn new(needs: Box<[RuleDerivedNeeds]>) -> Self {
        let graph_rules = needs
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.graph == GraphAdmission::Enabled)
            .map(|(rule, _)| RuleIndex(rule))
            .collect();
        Self {
            relations: needs
                .into_vec()
                .into_iter()
                .map(|rule| rule.relations)
                .collect(),
            graph_rules,
        }
    }
    pub(crate) fn relations_for(&self, rule: RuleIndex) -> Result<&[Relation], OperatingError> {
        tracing::trace!(target: activation::TARGET, event = activation::RELATION_RULE, rule = rule.0);
        self.relations
            .get(rule.0)
            .map(AsRef::as_ref)
            .ok_or(OperatingError::NotRun)
    }
    pub(crate) fn graph_for(&self, rule: RuleIndex) -> Result<GraphAdmission, OperatingError> {
        tracing::trace!(target: activation::TARGET, event = activation::GRAPH_RULE, rule = rule.0);
        let eligible = self.graph_rules.contains(&rule);
        if eligible {
            tracing::trace!(target: activation::TARGET, event = activation::GRAPH_HIT);
        }
        if rule.0 >= self.relations.len() {
            return Err(OperatingError::NotRun);
        }
        Ok(if eligible {
            GraphAdmission::Enabled
        } else {
            GraphAdmission::Disabled
        })
    }
}
