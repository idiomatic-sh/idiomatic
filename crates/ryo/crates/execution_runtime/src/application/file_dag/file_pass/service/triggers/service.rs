//! What the open file holds of the program's triggers: its root kinds, from the kind lists `fill_roots` made, with no source
//! read; and each node's activity, composed where it is first asked from the file's one read of each relation it needs.
use crate::application::file_dag::fact_lookup::service::FactLookup;
use crate::application::file_dag::file_pass::types::{
    FileLeaves, FileRoots, PassInputs, ReadCompleteness,
};
use ryo_execution_domain::domain::executable_program::activation::service::activity;
use ryo_execution_domain::domain::executable_program::activation::types::{
    NodeActivity, TriggerPresence, Triggers,
};
use ryo_execution_domain::domain::executable_program::service::activation;
use ryo_operators::domain::physical_plan::types::PlanNodeId;

/// Writes into `triggers` which root kinds the open file holds, by their place in `roots` (the plan's kind order): a kind where
/// a node it starts has a tree node of it. A file read in part holds every kind.
pub(crate) fn fill(triggers: &mut Triggers, roots: &FileRoots, lookup: &FactLookup<'_, '_>) {
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
    if !lookup.partials().is_empty() {
        triggers.kinds.fill(TriggerPresence::Present);
    }
}

/// Whether `node` runs in the open file of `inputs`: its state composed where it is first asked, each fact relation it needs
/// answered from the file's one read of it, or present unread in a file read in part.
pub(crate) fn node_activity(inputs: &PassInputs<'_, '_, '_>, node: PlanNodeId) -> NodeActivity {
    let mut leaves = FileLeaves {
        reads: &inputs.reads,
        lookup: inputs.lookup,
        completeness: if inputs.lookup.partials().is_empty() {
            ReadCompleteness::Whole
        } else {
            ReadCompleteness::InPart
        },
    };
    match activity(
        activation(inputs.program),
        inputs.triggers,
        &mut leaves,
        &mut inputs.activities.borrow_mut(),
        node,
    ) {
        Ok(answer) => answer,
        // Triggers of another program cannot show a node idle: it runs.
        Err(_) => NodeActivity::Active,
    }
}
