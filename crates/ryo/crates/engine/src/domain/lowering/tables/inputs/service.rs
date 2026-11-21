//! The services of the `inputs` part: what a step reads of an input. A relation is one atom; a step is its own branches, taken by its one reader, or one use of its definition: when it has several readers across the pack, when the reader wants one, or when it is past a bound.

use crate::domain::lowered::types::Emptiness;
use crate::domain::lowering::costing::service::branches_rows;
use crate::domain::lowering::definitions::service::{
    defined, fresh_cells, fresh_variable, regex, text, use_branch,
};
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::{copied, relation_read};
use crate::domain::lowering::flattening::constants::{BRANCH_ATOMS, INPUT_BRANCHES};
use crate::domain::lowering::flattening::service::compared;
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, RowsPerFile,
};
use crate::domain::lowering::nodes::service::{digest_of, input_shape, readers};
use crate::domain::lowering::nodes::types::{AdmittedRule, NodeHome};
use crate::domain::lowering::tables::service::test_of;
use crate::domain::lowering::tables::types::{InputWant, RuleFlattening};
use crate::domain::steps::types::{DefinitionId, TestOperation};
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::{
    Direction, Input, Operand, PlanComparison, QueryLiteral, StepId,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// The rule being flattened.
pub(crate) fn rule<'a>(state: &RuleFlattening<'a>) -> Option<&'a AdmittedRule<'a>> {
    state.rules.get(state.place.0)
}

/// The term of an operand over a row: a column's term, a literal, the value a parameter is bound to.
pub(crate) fn term(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    row: &[FlatTerm],
    operand: &Operand,
) -> Option<FlatTerm> {
    match operand {
        Operand::Column(column) => row.get(usize::from(*column)).copied(),
        Operand::Literal(QueryLiteral::Number(number)) => Some(FlatTerm::Number(*number)),
        Operand::Literal(QueryLiteral::Text(written)) => {
            Some(FlatTerm::Text(text(drafts, written)))
        }
        Operand::Parameter(parameter) => {
            let bindings = rule(state)?.rule.bindings.0;
            match bindings.get(usize::try_from(parameter.0).ok()?)? {
                ParameterBinding::Number(number) => Some(FlatTerm::Number(Integer(number.0))),
                ParameterBinding::Text(written) => Some(FlatTerm::Text(text(drafts, written))),
                ParameterBinding::Flag(_)
                | ParameterBinding::Numbers(_)
                | ParameterBinding::Texts(_)
                | ParameterBinding::TextSets(_)
                | ParameterBinding::Tables(_) => None,
            }
        }
        Operand::Function(_) => None,
    }
}

/// A branch with every comparison added, each over the branch's own row. Outer none: a comparison cannot be written; inner
/// none: the branch holds of no row.
pub(crate) fn selected(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    mut branch: FlatBranch,
    comparisons: &[PlanComparison],
) -> Option<Option<FlatBranch>> {
    for comparison in comparisons {
        let left = term(state, drafts, &branch.row, &comparison.left)?;
        let right = term(state, drafts, &branch.row, &comparison.right)?;
        let test = match (test_of(comparison.test)?, &comparison.right) {
            (TestOperation::Matches(_), Operand::Literal(QueryLiteral::Text(source))) => {
                TestOperation::Matches(regex(drafts, source))
            }
            (test, _) => test,
        };
        match compared(branch, test, left, right) {
            Some(kept) => branch = kept,
            None => return Some(None),
        }
    }
    Some(Some(branch))
}

/// Every branch with the comparisons added; the branches that hold of no row are dropped.
pub(crate) fn all_selected(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    branches: Vec<FlatBranch>,
    comparisons: &[PlanComparison],
) -> Option<Vec<FlatBranch>> {
    let mut kept = Vec::with_capacity(branches.len());
    for branch in branches {
        kept.extend(selected(state, drafts, branch, comparisons)?);
    }
    Some(kept)
}

/// One use of the definition of a step, made from the step's flattening the first time.
pub(crate) fn use_of(drafts: &mut Drafts, definition: DefinitionId) -> Flattened {
    Flattened {
        branches: vec![use_branch(drafts, definition)],
        emptiness: Emptiness::Definition(definition),
        keyless: Vec::new(),
    }
}

/// Whether a flattening is past a bound of one input: its branches, or the atoms of one of them.
pub(crate) fn past_bounds(flattened: &Flattened) -> bool {
    flattened.branches.len() > INPUT_BRANCHES
        || flattened
            .branches
            .iter()
            .any(|branch| branch.atoms.len() > BRANCH_ATOMS)
}

/// The definition of a step of the rule: found when the node has one, else made from the step's flattening.
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
    let flattened = state.flat.get_mut(place)?.take()?;
    let shape = rule.shapes.get(place)?.clone()?;
    let home = state.store.0.get(&digest).map_or(
        NodeHome {
            rule: state.place,
            step,
        },
        |node| node.home,
    );
    let rows = branches_rows(drafts, &flattened.branches);
    Some(defined(drafts, digest, home, shape, flattened, rows))
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
            let cut = want == InputWant::Defined
                || shared
                || always_defined(state, step)
                || waiting.is_some_and(past_bounds);
            if cut || drafts.defined.contains_key(&digest) {
                let definition = step_defined(state, drafts, step)?;
                Some(use_of(drafts, definition))
            } else {
                state.flat.get_mut(usize::from(step))?.take()
            }
        }
        Input::BoundObservation(_) => None,
    }
}

/// The same read with fresh variables, when a flattening is one read of a made table or of a definition and its row is
/// that read's cells: what each reader of such a step holds.
pub(crate) fn aliased(drafts: &mut Drafts, flattened: &Flattened) -> Option<Flattened> {
    let [branch] = flattened.branches.as_slice() else {
        return None;
    };
    let [
        FlatAtom::Rows {
            rows: rows @ (AtomRows::Made(_) | AtomRows::Definition(_)),
            cells,
            per_file,
        },
    ] = branch.atoms.as_slice()
    else {
        return None;
    };
    if *cells != branch.row {
        return None;
    }
    let fresh: Vec<FlatTerm> = cells
        .iter()
        .map(|cell| match cell {
            FlatTerm::Variable(_) => FlatTerm::Variable(fresh_variable(drafts)),
            FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File => *cell,
        })
        .collect();
    Some(Flattened {
        branches: vec![FlatBranch {
            atoms: vec![FlatAtom::Rows {
                rows: *rows,
                cells: fresh.clone(),
                per_file: *per_file,
            }],
            row: fresh,
        }],
        emptiness: copied(&flattened.emptiness),
        keyless: Vec::new(),
    })
}

/// The size of an input as its reader would hold it: its branches and the atoms of its largest; one of each for a relation
/// and for a step that is a definition.
pub(crate) fn sized(state: &RuleFlattening<'_>, drafts: &Drafts, read: Input) -> (usize, usize) {
    let Input::Step(step) = read else {
        return (1, 1);
    };
    let Some(rule) = rule(state) else {
        return (1, 1);
    };
    let digest = digest_of(rule, step);
    let waiting = state.flat.get(usize::from(step)).and_then(Option::as_ref);
    match waiting {
        Some(flattened)
            if !drafts.defined.contains_key(&digest)
                && readers(state.store, digest).0 < 2
                && !always_defined(state, step)
                && !past_bounds(flattened) =>
        {
            let atoms = flattened.branches.iter().map(|branch| branch.atoms.len());
            (flattened.branches.len(), atoms.max().unwrap_or(0))
        }
        Some(_) | None => (1, 1),
    }
}

/// Whether a step is always a definition for its readers: a nearest axis, whose walk a pipeline runs through a pattern of
/// the step's own definition.
pub(crate) fn always_defined(state: &RuleFlattening<'_>, step: StepId) -> bool {
    let operation = rule(state)
        .and_then(|rule| rule.rule.plan.steps.get(usize::from(step)))
        .map(|step| &step.operation);
    matches!(
        operation,
        Some(Operation::Axis {
            direction: Direction::NearestAncestor,
            ..
        })
    )
}
