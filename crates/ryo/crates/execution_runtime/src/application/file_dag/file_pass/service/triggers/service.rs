//! What the open file holds of the program's triggers: its root kinds, from the kind lists `fill_roots` made, with no source
//! read; and each fact relation's rows, the file's one read of it, made where a reader or a node's activity first asks.
use crate::application::file_dag::fact_lookup::service::FactLookup;
use crate::application::file_dag::file_pass::types::{FactRead, FileReads, FileRoots, PassInputs};
use crate::outbound::facts::errors::FactsError;
use ryo_execution_domain::domain::executable_program::activation::service::activity;
use ryo_execution_domain::domain::executable_program::activation::types::{
    FactAnswer, NodeActivity, TriggerPresence, Triggers,
};
use ryo_execution_domain::domain::executable_program::service::activation;
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_relations::domain::relations::types::{ColumnsView, Relation};
use std::cell::RefCell;

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

/// `relation`'s rows in the open file, or the lookup's refusal: the file's one read of it, made at its first ask and answered
/// to every later one, so a relation's read is one value for the whole file whichever reader asks first.
pub(crate) fn read<'p>(
    reads: &RefCell<FileReads<'p>>,
    lookup: &'p FactLookup<'_, '_>,
    relation: Relation,
) -> Result<ColumnsView<'p>, FactsError> {
    if let Some(known) = reads
        .borrow()
        .0
        .iter()
        .find(|read| read.relation == relation)
    {
        return known.read.clone();
    }
    let answered = lookup.columns(relation);
    reads.borrow_mut().0.push(FactRead {
        relation,
        read: answered.clone(),
    });
    answered
}

/// Whether `node` runs in the open file of `inputs`: its state composed where it is first asked, each fact relation it needs
/// answered from the file's one read of it, or present unread in a file read in part.
pub(crate) fn node_activity(inputs: &PassInputs<'_, '_, '_>, node: PlanNodeId) -> NodeActivity {
    let partial = !inputs.lookup.partials().is_empty();
    let mut facts = |relation: Relation| {
        if partial {
            return FactAnswer::Rows;
        }
        match read(&inputs.reads, inputs.lookup, relation) {
            Ok(view)
                if view
                    .columns
                    .first()
                    .is_some_and(|column| column.0.is_empty()) =>
            {
                FactAnswer::NoRow
            }
            // A relation of no column cannot show it has no row.
            Ok(_) => FactAnswer::Rows,
            Err(_) => FactAnswer::Refused,
        }
    };
    match activity(
        activation(inputs.program),
        inputs.triggers,
        &mut facts,
        &mut inputs.activities.borrow_mut(),
        node,
    ) {
        Ok(answer) => answer,
        // Triggers of another program cannot show a node idle: it runs.
        Err(_) => NodeActivity::Active,
    }
}
