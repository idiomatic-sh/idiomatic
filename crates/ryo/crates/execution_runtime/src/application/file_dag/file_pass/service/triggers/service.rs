//! The open file's triggers: each root kind's presence from the kind lists `fill_roots` made, each demanded fact relation's
//! from its rows; a file that cannot say what it holds holds every trigger.
use crate::application::file_dag::fact_lookup::service::FactLookup;
use crate::application::file_dag::file_pass::types::{FileRoots, RefusedRead};
use ryo_execution_domain::domain::executable_program::activation::service::trigger_facts;
use ryo_execution_domain::domain::executable_program::activation::types::{
    TriggerPresence, Triggers,
};
use ryo_execution_domain::domain::executable_program::service::activation;
use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
use ryo_relations::domain::relations::types::Relation;

/// Writes into `triggers` what the open file holds: a root kind, by its place in `roots` (the plan's kind order), where a node
/// it starts has a tree node of it; a fact relation where no reader of it runs (it is not in `demand`) or it has a row. A
/// partial read, or a demanded relation the lookup refuses, makes every trigger present, so every node runs and meets the
/// partiality or the refusal where it did before; the refusal is answered back, so its readers do not ask the lookup again.
pub(crate) fn fill(
    triggers: &mut Triggers,
    program: &ExecutableProgram,
    roots: &FileRoots,
    lookup: &FactLookup<'_, '_>,
    demand: &[Relation],
) -> Option<RefusedRead> {
    triggers.kinds.clear();
    triggers.kinds.extend(roots.kinds.iter().map(|root| {
        let held = root.nodes.iter().any(|node| {
            roots
                .lists
                .get(usize::try_from(node.0).unwrap_or(usize::MAX))
                .is_some_and(|list| !list.is_empty())
        });
        if held {
            TriggerPresence::Present
        } else {
            TriggerPresence::Absent
        }
    }));
    triggers.facts.clear();
    let mut refused = None;
    for relation in trigger_facts(activation(program)) {
        let presence = if refused.is_some() || !demand.contains(relation) {
            TriggerPresence::Present
        } else {
            match lookup.columns(*relation) {
                // A relation of no column cannot show it has no row.
                Ok(view)
                    if view
                        .columns
                        .first()
                        .is_some_and(|column| column.0.is_empty()) =>
                {
                    TriggerPresence::Absent
                }
                Ok(_) => TriggerPresence::Present,
                Err(error) => {
                    refused = Some(RefusedRead {
                        relation: *relation,
                        error,
                    });
                    TriggerPresence::Present
                }
            }
        };
        triggers.facts.push(presence);
    }
    if refused.is_some() || !lookup.partials().is_empty() {
        triggers.kinds.fill(TriggerPresence::Present);
        triggers.facts.fill(TriggerPresence::Present);
    }
    refused
}
