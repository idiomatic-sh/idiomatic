//! The services of the `inputs` part: what a step reads of an input. A relation is one atom; a step is its own branches, taken by its one reader, or one use of its definition: when it has several readers across the pack, when the reader wants one, or when it is past a bound.

use crate::domain::lowered::types::Emptiness;
use crate::domain::lowering::costing::service::branches_rows;
use crate::domain::lowering::definitions::service::{
    defined, fresh_cells, fresh_variable, text, use_branch,
};
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::{copied, relation_read};
use crate::domain::lowering::flattening::constants::{BRANCH_ATOMS, INPUT_BRANCHES};
use crate::domain::lowering::flattening::service::function_call;
use crate::domain::lowering::flattening::service::{own_walks, picked};
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, OwnWalks, RowColumn, RowsPerFile,
};
use crate::domain::lowering::nodes::service::{digest_of, input_shape, kept_columns, readers};
use crate::domain::lowering::nodes::types::{AdmittedRule, NodeHome};
use crate::domain::lowering::tables::service::tree_relation;
use crate::domain::lowering::tables::shapes::service::picked as kinds_picked;
use crate::domain::lowering::tables::types::InputSize;
use crate::domain::lowering::tables::types::{InputWant, RuleFlattening};
use crate::domain::lowering::tables::views::service::{branch_again, view_noted, view_read};
use crate::domain::steps::types::DefinitionId;
use ryo_relations::domain::relations::types::Count;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::{Direction, Input, Operand, QueryLiteral, StepId};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// The rule being flattened.
pub(crate) fn rule<'a>(state: &RuleFlattening<'a>) -> Option<&'a AdmittedRule<'a>> {
    state.rules.get(state.place.0)
}

/// The term of an operand over a branch's row: a column's term, a literal, the value a parameter is bound to, or the cell a
/// function gives, whose call joins the branch.
pub(crate) fn term(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    branch: &mut FlatBranch,
    operand: &Operand,
) -> Option<FlatTerm> {
    match operand {
        Operand::Column(column) => branch.row.get(usize::from(*column)).copied(),
        Operand::Literal(QueryLiteral::Number(number)) => Some(FlatTerm::Number(*number)),
        Operand::Literal(QueryLiteral::Text(written)) => {
            Some(FlatTerm::Text(text(drafts, state.kinds, written)))
        }
        Operand::Parameter(parameter) => {
            let bindings = rule(state)?.rule.bindings.0;
            match bindings.get(usize::try_from(parameter.0).unwrap_or(usize::MAX))? {
                ParameterBinding::Number(number) => Some(FlatTerm::Number(Integer(number.0))),
                ParameterBinding::Text(written) => {
                    Some(FlatTerm::Text(text(drafts, state.kinds, written)))
                }
                ParameterBinding::Flag(_)
                | ParameterBinding::Numbers(_)
                | ParameterBinding::Texts(_)
                | ParameterBinding::TextSets(_)
                | ParameterBinding::Tables(_) => None,
            }
        }
        Operand::Function(applied) => {
            let meaning = function_call(applied.function)?;
            let mut arguments = Vec::with_capacity(applied.arguments.len());
            for argument in &applied.arguments {
                arguments.push(term(state, drafts, branch, argument)?);
            }
            let given = FlatTerm::Variable(fresh_variable(drafts));
            branch.atoms.push(FlatAtom::Called {
                meaning,
                arguments,
                outputs: vec![given],
            });
            Some(given)
        }
    }
}

/// One use of the definition of a step, made from the step's flattening the first time. Its row is the step's: where the
/// definition holds fewer columns than the step has, a column it does not hold is a variable nothing gives, which no
/// reader names (every reader of such a definition is a projection that keeps held columns alone).
pub(crate) fn use_of(drafts: &mut Drafts, definition: DefinitionId, width: Count) -> Flattened {
    let mut used = use_branch(drafts, definition);
    if let Some(held) = drafts.held.get(&definition).cloned() {
        let mut row: Vec<FlatTerm> = (0..width.0)
            .map(|_| FlatTerm::Variable(fresh_variable(drafts)))
            .collect();
        for (cell, column) in used.row.iter().zip(held) {
            if let Some(slot) = row.get_mut(column.0) {
                *slot = *cell;
            }
        }
        used.row = row;
    }
    Flattened {
        branches: vec![used],
        emptiness: Emptiness::Definition(definition),
        keyless: Vec::new(),
    }
}

/// How an input past a bound is asked for: its branches, or the atoms of one of them, past the bounds of one input make it
/// one use of its definition.
pub(crate) fn bounded(flattened: &Flattened) -> InputWant {
    let past = flattened.branches.len() > INPUT_BRANCHES
        || flattened
            .branches
            .iter()
            .any(|branch| branch.atoms.len() > BRANCH_ATOMS);
    if past {
        InputWant::Defined
    } else {
        InputWant::Inline
    }
}

/// The definition of a step of the rule: found when the node has one, else made from the step's flattening. A node whose
/// every reader across the pack is a projection is defined on the columns those projections keep together: no reader
/// asks the others, so no table of it writes them and no pattern of it reads them. A step that holds a walk no stage runs
/// keeps its whole row: the pattern of its own definition its pipeline uses names the walk's ends by their columns.
pub(crate) fn step_defined(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    step: StepId,
) -> Option<DefinitionId> {
    let rule = rule(state)?;
    let digest = digest_of(rule, step);
    if let Some(definition) = drafts.defined.get(&digest) {
        return Some(*definition);
    }
    let place = usize::from(step);
    let mut flattened = state.flat.get_mut(place)?.take()?;
    let mut shape = rule.shapes.get(place)?.clone()?;
    let home = state.store.0.get(&digest).map_or(
        NodeHome {
            rule: state.place,
            step,
        },
        |node| node.home,
    );
    let kept = kept_columns(state.store, digest, Count(shape.0.len()))
        .filter(|_| own_walks(&flattened.branches) == OwnWalks::Stageable);
    let held: Option<Vec<RowColumn>> = kept.as_ref().map(|kept| {
        let columns = kept.iter().map(|column| RowColumn(usize::from(*column)));
        columns.collect()
    });
    if let (Some(kept), Some(held)) = (&kept, &held) {
        for branch in &mut flattened.branches {
            branch.row = picked(&branch.row, held.iter().copied())?;
        }
        shape = kinds_picked(&shape, kept)?;
    }
    let rows = branches_rows(drafts, &flattened.branches);
    let definition = defined(drafts, digest, home, shape, flattened, rows);
    if let Some(held) = held {
        let _new = drafts.held.insert(definition, held);
    }
    Some(definition)
}

/// What a step reads of an input. A relation is one branch of one atom. A step is its own branches, taken, unless it is a
/// definition: because it has one already, because the reader wants one, because it has several readers across the pack, or
/// because its branches are past a bound.
pub(crate) fn input(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    read: Input,
    want: InputWant,
) -> Option<Flattened> {
    match read {
        Input::Relation(relation) => {
            let _new = drafts.reads.insert(relation);
            let shape = input_shape(&[], read)?;
            let cells = fresh_cells(drafts, &shape);
            Some(Flattened {
                branches: vec![FlatBranch {
                    atoms: vec![FlatAtom::Rows {
                        rows: AtomRows::Relation(relation),
                        cells: cells.clone(),
                        per_file: RowsPerFile::Many,
                    }],
                    row: cells,
                }],
                emptiness: relation_read(relation),
                keyless: Vec::new(),
            })
        }
        Input::Step(step) => {
            let digest = digest_of(rule(state)?, step);
            let shared = readers(state.store, digest).0 > 1;
            let waiting = state.flat.get(usize::from(step))?.as_ref();
            // A step that is one read of a made table or of a definition is that read for each of its readers: a table
            // of a table's rows would be a copy.
            if let Some(alias) = waiting.and_then(|flattened| aliased(drafts, flattened)) {
                return Some(alias);
            }
            // A step that is one read of a relation with its pins is that read for each reader that takes its
            // branches: a table of a relation's rows would be a copy. A reader that asks a definition has one.
            view_noted(drafts, digest, waiting);
            if want == InputWant::Inline
                && let Some(read) = view_read(drafts, digest)
            {
                return Some(read);
            }
            let cut = want == InputWant::Defined
                || shared
                || own_want(state, step) == InputWant::Defined
                || waiting.is_some_and(|flattened| bounded(flattened) == InputWant::Defined);
            if cut || drafts.defined.contains_key(&digest) {
                let width = input_shape(&rule(state)?.shapes, read)?.0.len();
                let definition = step_defined(state, drafts, step)?;
                Some(use_of(drafts, definition, Count(width)))
            } else {
                state.flat.get_mut(usize::from(step))?.take()
            }
        }
        Input::BoundObservation(_) => None,
    }
}

/// The same read with fresh variables, when a flattening is one read of a made table or of a definition, its row is
/// that read's cells, and no two cells hold one variable: what each reader of such a step holds, and a reader that asks
/// a definition is given the source's own. A read that holds two of its columns equal (one variable in two cells) is
/// more than its source's rows: it is no alias, and keeps its own branch or its own definition.
pub(crate) fn aliased(drafts: &mut Drafts, flattened: &Flattened) -> Option<Flattened> {
    let [branch] = flattened.branches.as_slice() else {
        return None;
    };
    let [
        FlatAtom::Rows {
            rows: AtomRows::Made(_) | AtomRows::Definition(_),
            cells,
            ..
        },
    ] = branch.atoms.as_slice()
    else {
        return None;
    };
    let mut each = cells.iter().enumerate();
    let twice = each.any(|(place, cell)| {
        cells
            .iter()
            .skip(place.saturating_add(1))
            .any(|later| later == cell)
    });
    if *cells != branch.row || twice {
        return None;
    }
    Some(Flattened {
        branches: vec![branch_again(drafts, branch)],
        emptiness: copied(&flattened.emptiness),
        keyless: Vec::new(),
    })
}

/// The size of an input as its reader would hold it: one branch of one atom for a relation and for a step that is a
/// definition, else the step's own branches.
pub(crate) fn sized(state: &RuleFlattening<'_>, drafts: &Drafts, read: Input) -> InputSize {
    let one = InputSize {
        branches: Count(1),
        atoms: Count(1),
    };
    let (Input::Step(step), Some(rule)) = (read, rule(state)) else {
        return one;
    };
    let digest = digest_of(rule, step);
    let waiting = state.flat.get(usize::from(step)).and_then(Option::as_ref);
    match waiting {
        Some(flattened)
            if !drafts.defined.contains_key(&digest)
                && readers(state.store, digest).0 < 2
                && own_want(state, step) == InputWant::Inline
                && bounded(flattened) == InputWant::Inline =>
        {
            let atoms = flattened.branches.iter().map(|branch| branch.atoms.len());
            InputSize {
                branches: Count(flattened.branches.len()),
                atoms: Count(atoms.max().unwrap_or(0)),
            }
        }
        Some(_) | None => one,
    }
}

/// How a step is asked for whatever its reader wants: a nearest axis and a closure are always definitions, since a pipeline
/// runs their walk through a pattern of the step's own definition.
pub(crate) fn own_want(state: &RuleFlattening<'_>, step: StepId) -> InputWant {
    let operation = rule(state)
        .and_then(|rule| rule.rule.plan.steps.get(usize::from(step)))
        .map(|step| &step.operation);
    match operation {
        Some(
            Operation::Axis {
                direction: Direction::NearestAncestor,
                ..
            }
            | Operation::Closure { .. },
        ) => InputWant::Defined,
        // An ancestor axis over edges that are no tree is a closure from its start.
        Some(Operation::Axis {
            direction: Direction::Ancestor,
            edges,
            parent,
            child,
            ..
        }) if tree_relation(edges, *parent, *child).is_none() => InputWant::Defined,
        Some(_) | None => InputWant::Inline,
    }
}
