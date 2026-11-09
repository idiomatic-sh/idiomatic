//! Finite request descriptors; no executable Plan or mutable worker pool survives.
use crate::application::file_dag::derived::constants as activation;
use ryo_execution_domain::domain::{coverage::types::RuleIndex, operating::errors::OperatingError};
use ryo_relations::domain::relations::types::Relation;

use crate::application::file_dag::derived::owner::cache::types::GraphAdmission;
/// The derived needs of a compiled request: the relations each rule reads and the rules that walk a graph.
#[derive(Debug)]
pub struct RequestDerived {
    relations: Box<[Box<[Relation]>]>,
    graph_rules: rustc_hash::FxHashSet<RuleIndex>,
}
/// What one rule of a request needs of the derived views: the relations it reads per file, each once in the order first read, and
/// whether it walks a graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleDerivedNeeds {
    /// The relations the rule reads per file.
    pub relations: Box<[Relation]>,
    /// Whether the rule walks a graph.
    pub walks_graph: bool,
}
impl RequestDerived {
    /// The derived needs of a request, one per rule in rule order.
    #[must_use]
    pub fn new(needs: Box<[RuleDerivedNeeds]>) -> Self {
        let graph_rules = needs
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.walks_graph)
            .map(|(rule, _)| RuleIndex(rule))
            .collect();
        Self {
            relations: needs.into_vec().into_iter().map(|rule| rule.relations).collect(),
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
