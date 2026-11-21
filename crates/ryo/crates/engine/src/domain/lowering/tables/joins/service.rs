//! The services of the `joins` part: the branches of a join, of an antijoin and of an axis.

use crate::domain::lowered::types::Emptiness;
use crate::domain::lowering::definitions::service::fresh_variable;
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::derived;
use crate::domain::lowering::flattening::constants::{BRANCH_ATOMS, PRODUCT_BRANCHES};
use crate::domain::lowering::flattening::service::made_column;
use crate::domain::lowering::flattening::service::{picked, unified};
use crate::domain::lowering::flattening::types::EdgeColumns;
use crate::domain::lowering::flattening::types::{
    AtomRows, EqualTerms, FlatAtom, FlatTerm, RowColumn,
};
use crate::domain::lowering::flattening::types::{NearestDomain, NearestOver};
use crate::domain::lowering::nodes::service::input_shape;
use crate::domain::lowering::tables::closures::service::edge_rows;
use crate::domain::lowering::tables::inputs::service::{
    all_selected, input, rule, selection_emptiness, sized,
};
use crate::domain::lowering::tables::service::{tree_relation, walk_of};
use crate::domain::lowering::tables::types::InputSize;
use crate::domain::lowering::tables::types::JoinWants;
use crate::domain::lowering::tables::types::KeyPair;
use crate::domain::lowering::tables::types::{InputWant, RuleFlattening, TreeAxis};
use crate::domain::steps::types::{AxisDirection, Polarity};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, JoinColumn, Side, StepId};
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
        view,
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
    // The keys and the answer name the right side's columns through its view, when it has one.
    if !view.is_empty() {
        for branch in &mut rights.branches {
            let seen = view.iter().map(|column| RowColumn(usize::from(*column)));
            branch.row = picked(&branch.row, seen)?;
        }
    }
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
    // A step's rows are its definition's; a relation's own rows are asked in place.
    let rows = match (rights.emptiness, right) {
        (Emptiness::Definition(definition), _) => AtomRows::Definition(definition),
        (_, Input::Relation(relation)) => AtomRows::Relation(*relation),
        (_, Input::Step(_) | Input::BoundObservation(_)) => return None,
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
        // A relation asked with no cell within the file is asked whether it holds a row at all: not lowered here.
        if matches!(rows, AtomRows::Relation(_)) && cells.iter().all(Option::is_none) {
            return None;
        }
        branch.atoms.push(FlatAtom::Decided {
            polarity: Polarity::Absent,
            rows,
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
        domain_view,
        ..
    } = operation
    else {
        return None;
    };
    let tree = tree_relation(edges, *parent, *child);
    // The entity column is named through the domain's view.
    let entity = usize::from(view_source(domain_view, *domain_id)?);
    let start = usize::from(*id);
    match walk_of(*direction)? {
        TreeAxis::Nearest => {
            // The start's rows are a definition: the pattern a pipeline uses for the walk is then keyed by the start.
            let mut starts = input(state, drafts, *from, InputWant::Defined)?;
            let mut held = input(state, drafts, *domain, InputWant::Defined)?;
            // A step's rows are its definition's, asked by the element. A relation's own rows are read in place by the
            // element, with the tests of the domain's filter.
            let stops = match (&held.emptiness, domain) {
                (Emptiness::Definition(definition), _) => NearestDomain::Held {
                    definition: *definition,
                    column: made_column(RowColumn(entity))?,
                },
                (_, Input::Relation(_)) => {
                    let read = std::mem::take(&mut held.branches);
                    let mut read = all_selected(state, drafts, read, domain_filter)?;
                    let (Some(only), None) = (read.pop(), read.pop()) else {
                        return None;
                    };
                    let emptiness = std::mem::take(&mut held.emptiness);
                    held.emptiness =
                        selection_emptiness(state, drafts, *domain, domain_filter, emptiness);
                    NearestDomain::Read {
                        entity: *only.row.get(entity)?,
                        atoms: only.atoms,
                    }
                }
                (_, Input::Step(_) | Input::BoundObservation(_)) => return None,
            };
            // Edges that are no tree of a node relation are asked by the child for the parent.
            let over = match tree {
                Some(relation) => NearestOver::Tree(relation),
                None => NearestOver::Derived(EdgeColumns {
                    rows: edge_rows(state, drafts, *edges)?,
                    keys: Vec::new(),
                    source: made_column(RowColumn(usize::from(*child)))?,
                    destination: made_column(RowColumn(usize::from(*parent)))?,
                }),
            };
            for branch in &mut starts.branches {
                let reached = FlatTerm::Variable(fresh_variable(drafts));
                branch.atoms.push(FlatAtom::Nearest {
                    over: over.clone(),
                    domain: stops.clone(),
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
            // Edges that are no tree are climbed by an ancestor axis alone: one or more edges from the child to the
            // parent, a closure from the start.
            let climbed = match (tree, walk) {
                (Some(_), _) => None,
                (None, AxisDirection::Ancestor) => Some(EdgeColumns {
                    rows: edge_rows(state, drafts, *edges)?,
                    keys: Vec::new(),
                    source: made_column(RowColumn(usize::from(*child)))?,
                    destination: made_column(RowColumn(usize::from(*parent)))?,
                }),
                (
                    None,
                    AxisDirection::Parent
                    | AxisDirection::Descendant
                    | AxisDirection::NextSibling
                    | AxisDirection::PreviousSibling,
                ) => return None,
            };
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
                    let from = *left.row.get(start)?;
                    walked.atoms.push(match (tree, &climbed) {
                        (Some(relation), _) => FlatAtom::Walk {
                            relation,
                            direction: walk,
                            start: from,
                            reached,
                        },
                        (None, Some(edges)) => FlatAtom::Closure {
                            edges: edges.clone(),
                            keys: Vec::new(),
                            source: from,
                            reached,
                        },
                        (None, None) => return None,
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
