//! The services of the `views` part: whether a step's branches are one read of a relation with its pins, and that read
//! again for each reader. The rule (scope-engine3 2.2 names "light definitions inlined" among what is not built; this is
//! the narrower rule the measurement of F401 asked for: one relation, no join, no fan-out):
//!
//! A step is a view of a relation when its flattening is ONE branch whose ONE source is a relation's row, whose every
//! other atom is a pin of a cell of that row (an equality with a constant), and whose row holds cells of that row alone:
//! the relation's rows unchanged, projected, renamed, or held to constants. A reader that takes the step's branches
//! (`InputWant::Inline`) is given the read itself with fresh variables, whatever the number of the step's readers: it
//! scans the relation, reads the bucket a pin names, or probes by its own key, and a pin no access is keyed by is a test
//! of its own pipeline or program. No definition and no table is made for those readers.
//!
//! What stays a definition, and so a table when it is read with nothing bound: a step some reader asks as a definition
//! (`InputWant::Defined`: the rows of a fill, which must be distinct, the right side of an absence, a nearest walk's start
//! and domain, a closure's edges), for that reader alone; a step with a test that is no pin (a comparison, a pattern, a
//! call's cell: a filter by what is no constant of the read); a step of two branches (a union) or of two sources (a join).

use super::types::RelationView;
use crate::domain::lowering::definitions::service::fresh_variable;
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::copied;
use crate::domain::lowering::flattening::service::each_term;
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable,
};
use crate::domain::steps::types::TestOperation;
use ryo_values::domain::query_plans::types::NodeDigest;
use std::collections::HashMap;

/// The read a step's branches are, when they are one read of a relation with its pins. None for anything else: two
/// branches, a second source, an atom that is no pin of the read's own cells, a row that holds anything but those cells.
#[must_use]
pub(crate) fn relation_read(branches: &[FlatBranch]) -> Option<RelationView> {
    let [branch] = branches else {
        return None;
    };
    let mut read: Option<RelationView> = None;
    let mut pinned: Vec<FlatTerm> = Vec::new();
    for atom in &branch.atoms {
        match atom {
            FlatAtom::Rows {
                rows: AtomRows::Relation(relation),
                cells,
                ..
            } if read.is_none() => {
                read = Some(RelationView {
                    relation: *relation,
                    cells: cells.clone(),
                    row: branch.row.clone(),
                });
            }
            FlatAtom::Test {
                test: TestOperation::Equal,
                left: left @ FlatTerm::Variable(_),
                right: FlatTerm::Text(_) | FlatTerm::Number(_),
            } => pinned.push(*left),
            FlatAtom::Rows { .. }
            | FlatAtom::Test { .. }
            | FlatAtom::Latest { .. }
            | FlatAtom::Decided { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Closure { .. }
            | FlatAtom::Called { .. } => return None,
        }
    }
    let read = read?;
    let mut held = pinned.iter().chain(&read.row);
    held.all(|term| read.cells.contains(term)).then_some(read)
}

/// Notes a step as a view of a relation, the first time its flattening is met and is one: the read is kept for the pack,
/// by the step's node, so that every reader in every rule reads the relation in place, whichever reader came first and
/// whether some reader made the step a definition.
pub(crate) fn view_noted(drafts: &mut Drafts, node: NodeDigest, waiting: Option<&Flattened>) {
    let Some(flattened) = waiting.filter(|_| !drafts.views.contains_key(&node)) else {
        return;
    };
    if relation_read(&flattened.branches).is_some() {
        let view = Flattened {
            branches: flattened.branches.clone(),
            emptiness: copied(&flattened.emptiness),
            keyless: Vec::new(),
        };
        let _new = drafts.views.insert(node, view);
    }
}

/// A branch again, with a fresh variable for each of its variables: one for each VARIABLE, not one for each cell, so
/// that two cells that held one variable (two columns of a row held equal) still hold one, and a pin still pins its cell.
pub(crate) fn branch_again(drafts: &mut Drafts, branch: &FlatBranch) -> FlatBranch {
    let mut again = branch.clone();
    let mut fresh: HashMap<FlatVariable, FlatTerm> = HashMap::new();
    let mut renamed = |term: &mut FlatTerm| {
        if let FlatTerm::Variable(variable) = *term {
            *term = *fresh
                .entry(variable)
                .or_insert_with(|| FlatTerm::Variable(fresh_variable(drafts)));
        }
    };
    for atom in &mut again.atoms {
        each_term(atom, &mut renamed);
    }
    again.row.iter_mut().for_each(&mut renamed);
    again
}

/// The read of a step that is a view of a relation, with fresh variables: what each reader that takes the step's branches
/// holds. None for a step that is no view.
pub(crate) fn view_read(drafts: &mut Drafts, node: NodeDigest) -> Option<Flattened> {
    let view = drafts.views.get(&node)?;
    let emptiness = copied(&view.emptiness);
    let read = view.branches.clone();
    let branches = read.iter().map(|branch| branch_again(drafts, branch));
    Some(Flattened {
        branches: branches.collect(),
        emptiness,
        keyless: Vec::new(),
    })
}
