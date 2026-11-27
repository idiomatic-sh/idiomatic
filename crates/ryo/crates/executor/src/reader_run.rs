//! File requirements shared by the provider visit and executor.
use super::{finish, first};
use crate::languages as l;
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::service as catalog;

pub mod packet;

pub mod pairs;


pub fn file_requirements(
    plan: &ryo_values::domain::finish_plans::types::FinishPlan,
    language: &first::Language,
) -> Vec<(String, (bool, Vec<Vec<String>>, Vec<Vec<String>>))> {
    use ryo_values::domain::finish_plans::types::RuleEmit;
    assert!(plan.unplanned.is_empty(), "unplanned file queries");
    let mut skips = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let terms = super::pair_summary::conjuncts(step);
        skips.push(super::pair_skip::Skip::with_terms(step, &skips, language, &terms));
    }
    plan.rules
        .iter()
        .map(|rule| {
            let requirements = match &rule.emit {
                RuleEmit::Written(emit) => skips[emit.step.0].root(),
                RuleEmit::Unwritten => Default::default(),
            };
            (rule.rule.0.to_string(), requirements)
        })
        .collect()
}

/// SDK packets have node-addressable columns or file-wide rows.
pub fn fact_family(name: &str) -> super::pair_summary::cost::Family {
    use super::pair_summary::cost::Family;
    match catalog::relation_named(NameRef(name)) {
        Some(rel) if !catalog::kinds(rel).contains(&l::CellKind::Node) => Family::Sets,
        _ => Family::Fact,
    }
}
