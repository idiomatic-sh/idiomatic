//! The services of the `joins` part: the branches of a join, of an antijoin and of an axis.

use crate::domain::lowered::types::Emptiness;
use crate::domain::lowering::definitions::service::fresh_variable;
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::derived;
use crate::domain::lowering::flattening::constants::{BRANCH_ATOMS, PRODUCT_BRANCHES};
use crate::domain::lowering::flattening::service::made_column;
use crate::domain::lowering::flattening::service::{picked, unified};
use crate::domain::lowering::flattening::types::EdgeColumns;
use crate::domain::lowering::flattening::types::{EqualTerms, FlatAtom, FlatTerm, RowColumn};
use crate::domain::lowering::nodes::service::input_shape;
use crate::domain::lowering::tables::inputs::service::{all_selected, input, rule, sized};
use crate::domain::lowering::tables::service::{tree_relation, walk_of};
use crate::domain::lowering::tables::types::InputSize;
use crate::domain::lowering::tables::types::JoinWants;
use crate::domain::lowering::tables::types::KeyPair;
use crate::domain::lowering::tables::types::{InputWant, RuleFlattening, TreeAxis};
use crate::domain::steps::types::Polarity;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side, StepId};
use ryo_values::domain::plans::types::Operation;

/// How each side of a join is asked for: both inline while the product of their branches and the atoms of a joined branch
/// are within the bounds; else the larger side is one use of its definition, and both when that is not enough.
pub(crate) fn join_wants(left: InputSize, right: InputSize) -> JoinWants {
    let one = InputSize {
        branches: Count(1),
        atoms: Count(1),
    };
    let fits = |left: InputSize, right: InputSize| {
        left.branches.0.saturating_mul(right.branches.0) <= PRODUCT_BRANCHES
            && left.atoms.0.saturating_add(right.atoms.0) <= BRANCH_ATOMS
    };
    let weight = |side: InputSize| side.branches.0.saturating_mul(side.atoms.0);
    let (left, right) = if fits(left, right) {
        (InputWant::Inline, InputWant::Inline)
    } else if weight(right) >= weight(left) && fits(left, one) {
        (InputWant::Inline, InputWant::Defined)
    } else if weight(right) < weight(left) && fits(one, right) {
        (InputWant::Defined, InputWant::Inline)
    } else {
        (InputWant::Defined, InputWant::Defined)
    };
    JoinWants { left, right }
}

/// The answer row of a join over the row of its left side followed by the row of its right.
pub(crate) fn join_row(
    row: &[FlatTerm],
    left_width: Count,
    output: &[JoinColumn],
) -> Option<Vec<FlatTerm>> {
    picked(
        row,
        output.iter().map(|column| {
            RowColumn(match column.side {
                Side::Left => usize::from(column.column),
                Side::Right => left_width.0.saturating_add(usize::from(column.column)),
            })
        }),
    )
}

/// The keys of a join, each a column of its left side and a column of its right.
pub(crate) fn key_pairs<'k>(
    left_on: &'k [ColumnIndex],
    right_on: &'k [ColumnIndex],
) -> impl Iterator<Item = KeyPair> + 'k {
    let pairs = left_on.iter().zip(right_on);
    pairs.map(|(left, right)| KeyPair {
        left: RowColumn(usize::from(*left)),
        right: RowColumn(usize::from(*right)),
    })
}

/// The branches of a join.
pub(crate) fn join_arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    step: StepId,
    operation: &Operation,
) -> Option<Flattened> {
    let Operation::Join {
        left,
        right,
        left_on,
        right_on,
        output,
        filter,
        ..
    } = operation
    else {
        return None;
    };
    let shapes = &rule(state)?.shapes;
    let left_shape = input_shape(shapes, *left)?;
    let wants = join_wants(sized(state, drafts, *left), sized(state, drafts, *right));
    let lefts = input(state, drafts, *left, wants.left)?;
    let mut rights = input(state, drafts, *right, wants.right)?;
    rights.branches = all_selected(state, drafts, rights.branches, filter)?;
    let mut branches = Vec::with_capacity(lefts.branches.len() * rights.branches.len());
    for left_branch in &lefts.branches {
        for right_branch in &rights.branches {
            let width = left_branch.row.len();
            let mut pairs = Vec::with_capacity(left_on.len());
            for KeyPair {
                left: RowColumn(left_key),
                right: RowColumn(right_key),
            } in key_pairs(left_on, right_on)
            {
                pairs.push(EqualTerms {
                    left: *left_branch.row.get(left_key)?,
                    right: *right_branch.row.get(right_key)?,
                });
            }
            let mut joined = left_branch.clone();
            joined.atoms.extend(right_branch.atoms.iter().cloned());
            joined.row.extend(right_branch.row.iter().copied());
            if let Some(mut joined) = unified(joined, &pairs) {
                joined.row = join_row(&joined.row, Count(width), output)?;
                branches.push(joined);
            }
        }
    }
    // A join whose every key is the file's own column states no key within the file: its rows are a product.
    let keyless = key_pairs(left_on, right_on)
        .all(|key| left_shape.0.get(key.left.0) == Some(&CellKind::File));
    let mut stated = lefts.keyless;
    stated.extend(rights.keyless);
    if keyless {
        stated.push(step);
    }
    Some(Flattened {
        branches,
        emptiness: derived(operation, vec![lefts.emptiness, rights.emptiness]),
        keyless: stated,
    })
}

/// The branches of an antijoin: each left branch with one decision of the right side's definition.
pub(crate) fn antijoin_arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    operation: &Operation,
) -> Option<Flattened> {
    let Operation::Antijoin {
        left,
        right,
        left_on,
        right_on,
    } = operation
    else {
        return None;
    };
    let width = input_shape(&rule(state)?.shapes, *right)?.0.len();
    let mut lefts = input(state, drafts, *left, InputWant::Inline)?;
    let rights = input(state, drafts, *right, InputWant::Defined)?;
    let Emptiness::Definition(definition) = rights.emptiness else {
        return None;
    };
    for branch in &mut lefts.branches {
        let mut cells: Vec<Option<FlatTerm>> = vec![None; width];
        let mut twice = Vec::new();
        for KeyPair {
            left: RowColumn(left_key),
            right: RowColumn(right_key),
        } in key_pairs(left_on, right_on)
        {
            let asked = *branch.row.get(left_key)?;
            match cells.get_mut(right_key)? {
                // One right column asked for two left cells: the two are equal, and one is asked.
                Some(first) => twice.push((*first, asked)),
                cell @ None => *cell = Some(asked).filter(|term| *term != FlatTerm::File),
            }
        }
        branch.atoms.push(FlatAtom::Decided {
            polarity: Polarity::Absent,
            definition,
            cells,
        });
        // The absence is asked only where the two left cells are equal; where they differ no right row matches, and the
        // left row stays: that case has no branch form here, so such a plan is not lowered.
        if !twice.is_empty() {
            return None;
        }
    }
    let emptiness = derived(operation, vec![lefts.emptiness]);
    Some(Flattened { emptiness, ..lefts })
}

/// The branches of an axis over a node relation's tree. Along an axis: each start beside each row of the domain, one walk
/// between the two. To the nearest ancestor: each start with one nearest walk, whose condition is the domain's definition.
pub(crate) fn axis_arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    operation: &Operation,
) -> Option<Flattened> {
    let Operation::Axis {
        from,
        edges,
        domain,
        direction,
        id,
        parent,
        child,
        domain_id,
        domain_filter,
        ..
    } = operation
    else {
        return None;
    };
    let relation = tree_relation(edges, *parent, *child)?;
    let (start, entity) = (usize::from(*id), usize::from(*domain_id));
    match walk_of(*direction)? {
        TreeAxis::Nearest => {
            // The start's rows are a definition: the pattern a pipeline uses for the walk is then keyed by the start.
            let mut starts = input(state, drafts, *from, InputWant::Defined)?;
            let held = input(state, drafts, *domain, InputWant::Defined)?;
            let Emptiness::Definition(definition) = held.emptiness else {
                return None;
            };
            for branch in &mut starts.branches {
                let reached = FlatTerm::Variable(fresh_variable(drafts));
                branch.atoms.push(FlatAtom::Nearest {
                    relation,
                    domain: definition,
                    column: made_column(RowColumn(entity))?,
                    start: *branch.row.get(start)?,
                    reached,
                });
                branch.row.push(reached);
            }
            let parts = vec![starts.emptiness, Emptiness::Unknown, held.emptiness];
            Some(Flattened {
                emptiness: derived(operation, parts),
                ..starts
            })
        }
        TreeAxis::Along(walk) => {
            let wants = join_wants(sized(state, drafts, *from), sized(state, drafts, *domain));
            let starts = input(state, drafts, *from, wants.left)?;
            let mut held = input(state, drafts, *domain, wants.right)?;
            held.branches = all_selected(state, drafts, held.branches, domain_filter)?;
            let mut branches = Vec::with_capacity(starts.branches.len() * held.branches.len());
            for left in &starts.branches {
                for right in &held.branches {
                    let mut walked = left.clone();
                    let reached = *right.row.get(entity)?;
                    walked.atoms.extend(right.atoms.iter().cloned());
                    walked.atoms.push(FlatAtom::Walk {
                        relation,
                        direction: walk,
                        start: *left.row.get(start)?,
                        reached,
                    });
                    walked.row.push(reached);
                    branches.push(walked);
                }
            }
            let mut keyless = starts.keyless;
            keyless.extend(held.keyless);
            let parts = vec![starts.emptiness, Emptiness::Unknown, held.emptiness];
            Some(Flattened {
                branches,
                emptiness: derived(operation, parts),
                keyless,
            })
        }
    }
}

/// The branch of a closure: each row of the edges gives a source and its keys, and one walk reaches from that source
/// over one or more edges of the same keys. The edges are a definition; the walk asks it by a source, or by a destination
/// when it is entered from what is reached.
pub(crate) fn closure_arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    operation: &Operation,
) -> Option<Flattened> {
    let Operation::Closure {
        edges,
        keys,
        source,
        destination,
    } = operation
    else {
        return None;
    };
    let shape = input_shape(&rule(state)?.shapes, *edges)?;
    let mut read = input(state, drafts, *edges, InputWant::Defined)?;
    let Emptiness::Definition(definition) = read.emptiness else {
        return None;
    };
    let column = |place: &ColumnIndex| RowColumn(usize::from(*place));
    // The file's own column is no key within the file: the walk stays in the other keys.
    let within = keys
        .iter()
        .filter(|key| shape.0.get(usize::from(**key)) != Some(&CellKind::File));
    let walked = EdgeColumns {
        definition,
        keys: within
            .clone()
            .filter_map(|key| made_column(column(key)))
            .collect(),
        source: made_column(column(source))?,
        destination: made_column(column(destination))?,
    };
    for branch in &mut read.branches {
        let reached = FlatTerm::Variable(fresh_variable(drafts));
        let from = *branch.row.get(usize::from(*source))?;
        let stays = picked(&branch.row, within.clone().map(column))?;
        let mut row = picked(&branch.row, keys.iter().map(column))?;
        row.extend([from, reached]);
        branch.atoms.push(FlatAtom::Closure {
            edges: walked.clone(),
            keys: stays,
            source: from,
            reached,
        });
        branch.row = row;
    }
    Some(Flattened {
        emptiness: derived(operation, vec![Emptiness::Definition(definition)]),
        ..read
    })
}
