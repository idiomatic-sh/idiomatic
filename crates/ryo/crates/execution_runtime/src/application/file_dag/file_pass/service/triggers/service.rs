//! The open file's triggers: each root kind's presence from the kind lists `fill_roots` made, each demanded fact relation's
//! from its rows; a file that cannot say what it holds holds every trigger.
use crate::application::file_dag::fact_lookup::service::FactLookup;
use crate::application::file_dag::file_pass::types::{FactRead, FactReads, FileRoots};
use ryo_execution_domain::domain::executable_program::activation::service::trigger_facts;
use ryo_execution_domain::domain::executable_program::activation::types::{
    TriggerPresence, Triggers,
};
use ryo_execution_domain::domain::executable_program::service::activation;
use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
use ryo_relations::domain::relations::types::Relation;

/// Writes into `triggers` what the open file holds: a root kind, by its place in `roots` (the plan's kind order), where a node
/// it starts has a tree node of it; a fact relation where no reader of it runs (it is not in `demand`) or it has a row. A
/// file read in part holds every trigger and is read nothing here. A demanded relation the lookup refuses makes every trigger
/// present, so every node runs and meets the refusal where it did before; no other relation is read after it. Each read is
/// answered back, rows or refusal, as the first read of the relation in the file: its first reader answers it instead of
/// asking the lookup.
pub(crate) fn fill<'p>(
    triggers: &mut Triggers,
    program: &ExecutableProgram,
    roots: &FileRoots,
    lookup: &'p FactLookup<'_, '_>,
    demand: &[Relation],
) -> FactReads<'p> {
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
    let mut reads = FactReads::default();
    // A file read in part cannot say a relation has no row: every trigger is present, and nothing is read here.
    if !lookup.partials().is_empty() {
        triggers.kinds.fill(TriggerPresence::Present);
        triggers.facts.resize(
            trigger_facts(activation(program)).len(),
            TriggerPresence::Present,
        );
        return reads;
    }
    let mut refused = false;
    for relation in trigger_facts(activation(program)) {
        let presence = if refused || !demand.contains(relation) {
            TriggerPresence::Present
        } else {
            let read = lookup.columns(*relation);
            let presence = match &read {
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
                Err(_) => {
                    refused = true;
                    TriggerPresence::Present
                }
            };
            reads.0.push(FactRead {
                relation: *relation,
                read: std::cell::RefCell::new(Some(read)),
            });
            presence
        };
        triggers.facts.push(presence);
    }
    if refused {
        triggers.kinds.fill(TriggerPresence::Present);
        triggers.facts.fill(TriggerPresence::Present);
    }
    reads
}
