//! Compile the catalog's scope and absence dependencies from the already parsed root.
//! File and project evaluation use only the session executor. Its partial inputs retain
//! their typed origins and can support positive rows even where absence is undecidable.
use super::{Program, Result};
use ryo_execution_domain::domain::{
    coverage::{
        service::unknown_message,
        types::{CoverageScope, Partial},
    },
    trust::{
        service::{matters, scoped_needs},
        types::{Bearing, Needs},
    },
};
use ryo_language::domain::files::types::FilePath;
pub(crate) struct Contract {
    pub cross: bool,
    pub frontier: bool,
    pub needs: Needs,
}

/// Derive coverage from the bases reachable by this plan's outputs. Negative
/// operators trust only the inputs whose missing rows can change their answers.
pub(crate) fn describe(program: &Program, local: &[bool]) -> Result<Contract> {
    use ryo_execution_domain::domain::trust::types::relation_read::value::RelationRead;
    use std::collections::BTreeSet;
    let mut needs = Needs::default();
    let plan = &program.plan;
    let mut reached = vec![false; plan.nodes.len()];
    let mut negative_file = reached.clone();
    let mut negative_project = reached.clone();
    for (_, output, _, _) in &plan.emits { reached[*output] = true; }
    let cross = plan.emits.iter().any(|(_, index, _, _)| !local[*index]);
    let mut frontier = false;
    for (i, step) in plan.nodes.iter().enumerate().rev() {
        if !reached[i] { continue; }
        let sensitive = super::finish::session::absence_input(step).and_then(|(name, _)| {
            step.relation_input(name)
        });
        if let Some(source) = sensitive {
            if local[i] { negative_file[source] = true; } else { negative_project[source] = true; }
        }
        for &operand in &step.inputs {
            reached[operand] = true;
            if negative_file[i] { negative_file[operand] = true; }
            if negative_project[i] { negative_project[operand] = true; }
            frontier |= !local[i] && local[operand];
        }
    }
    let mut sets: [BTreeSet<super::l::Relation>; 3] = Default::default();
    for &(at, relation) in &program.base_steps {
        if let Some(relation) = relation {
            for (set, used) in sets.iter_mut().zip([reached[at], negative_file[at], negative_project[at]]) {
                if used { set.insert(relation); }
            }
        }
    }
    for (out, relations) in [&mut needs.reads, &mut needs.per_file, &mut needs.cross].into_iter().zip(sets) {
        out.extend(relations.into_iter().map(RelationRead::whole));
    }
    Ok(Contract { cross, frontier, needs })
}

impl Contract {
    pub(crate) fn report(&self, partials: &[Partial], path: &str) -> Vec<(String, String)> {
        let source = FilePath(path.into());
        partials
            .iter()
            .filter(|p| {
                self.needs
                    .reads
                    .iter()
                    .any(|r| matters(r, p) == Bearing::Matters)
            })
            .map(|p| {
                (
                    path.to_owned(),
                    unknown_message(p, CoverageScope::File(&source))
                        .0
                        .into_string(),
                )
            })
            .collect()
    }
}
