//! The services of the `arms` part: a rule's steps flattened, each once, in step order. A step's branches are taken by its one
//! reader; a step with several readers across the pack, the right side of an absence, the rows of a fill and an input past
//! a bound are each one definition, and their reader holds one use. The `match` over `Operation` names every operation.

use crate::domain::lowered::types::{Emptiness, FillInput, GroupFill, MadeFill, OrderedFill};
use crate::domain::lowering::costing::service::branches_rows;
use crate::domain::lowering::definitions::service::{
    defined, fresh_cells, fresh_variable, regex, text, text_word, use_branch,
};
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::{copied, derived, literal_read, relation_read};
use crate::domain::lowering::flattening::constants::CUT_BOUNDS;
use crate::domain::lowering::flattening::service::{compared, picked, unified};
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, RowsPerFile,
};
use crate::domain::lowering::nodes::service::{digest_of, input_shape, readers};
use crate::domain::lowering::nodes::types::{NodeHome, RuleNodes, StepShape};
use crate::domain::lowering::tables::fills::service::{MadeOf, made, scanned};
use crate::domain::lowering::tables::service::test_of;
use crate::domain::lowering::tables::types::{InputWant, RuleFlattening};
use crate::domain::steps::types::{
    AnchorKinds, ColumnList, DefinitionId, MadeColumn, MadeTableId, Polarity, TestOperation,
};
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::types::{CellKind, Integer, KindBits};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input, JoinColumn, Operand, PlanComparison, QueryLiteral, Side, StepId,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// The rule being flattened.
fn rule<'a>(state: &RuleFlattening<'a>) -> Option<&'a RuleNodes<'a>> {
    state.rules.get(state.place)
}

/// The term of an operand over a row: a column's term, a literal, the value a parameter is bound to.
fn term(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    row: &[FlatTerm],
    operand: &Operand,
) -> Option<FlatTerm> {
    match operand {
        Operand::Column(column) => row.get(usize::from(*column)).copied(),
        Operand::Literal(QueryLiteral::Number(number)) => Some(FlatTerm::Number(*number)),
        Operand::Literal(QueryLiteral::Text(written)) => {
            Some(FlatTerm::Text(text(drafts, &written.0)))
        }
        Operand::Parameter(parameter) => {
            let bindings = rule(state)?.rule.bindings.0;
            match bindings.get(usize::try_from(parameter.0).ok()?)? {
                ParameterBinding::Number(number) => Some(FlatTerm::Number(Integer(number.0))),
                ParameterBinding::Text(written) => Some(FlatTerm::Text(text(drafts, &written.0))),
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
fn selected(
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
                TestOperation::Matches(regex(drafts, &source.0))
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
fn all_selected(
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
fn use_of(drafts: &mut Drafts, definition: DefinitionId) -> Flattened {
    Flattened {
        branches: vec![use_branch(drafts, definition)],
        emptiness: Emptiness::Definition(definition),
        keyless: Vec::new(),
    }
}

/// Whether a flattening is past a bound of one input: its branches, or the atoms of one of them.
fn past_bounds(flattened: &Flattened) -> bool {
    flattened.branches.len() > CUT_BOUNDS.branches
        || flattened
            .branches
            .iter()
            .any(|branch| branch.atoms.len() > CUT_BOUNDS.atoms)
}

/// The definition of a step of the rule: found when the node has one, else made from the step's flattening.
fn step_defined(
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
fn input(
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
            let shared = readers(state.store, digest) > 1;
            let waiting = state.flat.get(usize::from(step))?.as_ref();
            // A step that is one read of a made table or of a definition is that read for each of its readers: a table
            // of a table's rows would be a copy.
            if let Some(alias) = waiting.and_then(|flattened| aliased(drafts, flattened)) {
                return Some(alias);
            }
            let cut = want == InputWant::Defined || shared || waiting.is_some_and(past_bounds);
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
fn aliased(drafts: &mut Drafts, flattened: &Flattened) -> Option<Flattened> {
    let [branch] = flattened.branches.as_slice() else {
        return None;
    };
    let [FlatAtom::Rows {
        rows: rows @ (AtomRows::Made(_) | AtomRows::Definition(_)),
        cells,
        per_file,
    }] = branch.atoms.as_slice()
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
fn sized(state: &RuleFlattening<'_>, drafts: &Drafts, read: Input) -> (usize, usize) {
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
                && readers(state.store, digest) < 2
                && !past_bounds(flattened) =>
        {
            let atoms = flattened.branches.iter().map(|branch| branch.atoms.len());
            (flattened.branches.len(), atoms.max().unwrap_or(0))
        }
        Some(_) | None => (1, 1),
    }
}

/// How each side of a join is asked for: both inline while the product of their branches and the atoms of a joined branch
/// are within the bounds; else the larger side is one use of its definition, and both when that is not enough.
fn join_wants(left: (usize, usize), right: (usize, usize)) -> (InputWant, InputWant) {
    let fits = |left: (usize, usize), right: (usize, usize)| {
        left.0.saturating_mul(right.0) <= CUT_BOUNDS.product
            && left.1.saturating_add(right.1) <= CUT_BOUNDS.atoms
    };
    if fits(left, right) {
        return (InputWant::Inline, InputWant::Inline);
    }
    let larger_right = right.0.saturating_mul(right.1) >= left.0.saturating_mul(left.1);
    match (larger_right, fits(left, (1, 1)), fits((1, 1), right)) {
        (true, true, _) => (InputWant::Inline, InputWant::Defined),
        (false, _, true) => (InputWant::Defined, InputWant::Inline),
        (true, false, _) | (false, _, false) => (InputWant::Defined, InputWant::Defined),
    }
}

/// The answer row of a join over the row of its left side followed by the row of its right.
fn join_row(row: &[FlatTerm], left_width: usize, output: &[JoinColumn]) -> Option<Vec<FlatTerm>> {
    picked(
        row,
        output.iter().map(|column| match column.side {
            Side::Left => usize::from(column.column),
            Side::Right => left_width.saturating_add(usize::from(column.column)),
        }),
    )
}

/// The keys of a join as pairs of places.
fn key_pairs<'k>(
    left_on: &'k [ColumnIndex],
    right_on: &'k [ColumnIndex],
) -> impl Iterator<Item = (usize, usize)> + 'k {
    let pairs = left_on.iter().zip(right_on);
    pairs.map(|(left, right)| (usize::from(*left), usize::from(*right)))
}

/// The emptiness of a selection: of a relation read with a literal in a column when that is what it is, else its input's.
fn selection_emptiness(
    state: &RuleFlattening<'_>,
    drafts: &Drafts,
    from: Input,
    comparisons: &[PlanComparison],
    read: Emptiness,
) -> Emptiness {
    let Input::Relation(relation) = from else {
        return read;
    };
    let listed = columns(relation);
    for comparison in comparisons {
        let (Operand::Column(column), Operand::Literal(QueryLiteral::Text(written))) =
            (&comparison.left, &comparison.right)
        else {
            continue;
        };
        let (Some(number), Some(column), Some(TestOperation::Equal)) = (
            drafts.text_numbers.get(&written.0),
            listed.get(usize::from(*column)),
            test_of(comparison.test),
        ) else {
            continue;
        };
        let place = state.kinds.kinds.iter().position(|kind| kind.0 == written.0);
        let kinds = place
            .and_then(|place| u16::try_from(place).ok())
            .map(|bits| AnchorKinds([KindBits(bits)].into()));
        return literal_read(relation, *column, text_word(*number), kinds);
    }
    read
}

/// The branches of a join.
fn join_arm(
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
    let lefts = input(state, drafts, *left, wants.0)?;
    let mut rights = input(state, drafts, *right, wants.1)?;
    rights.branches = all_selected(state, drafts, rights.branches, filter)?;
    let mut branches = Vec::with_capacity(lefts.branches.len() * rights.branches.len());
    for left_branch in &lefts.branches {
        for right_branch in &rights.branches {
            let width = left_branch.row.len();
            let mut pairs = Vec::with_capacity(left_on.len());
            for (left_key, right_key) in key_pairs(left_on, right_on) {
                pairs.push((*left_branch.row.get(left_key)?, *right_branch.row.get(right_key)?));
            }
            let mut joined = left_branch.clone();
            joined.atoms.extend(right_branch.atoms.iter().cloned());
            joined.row.extend(right_branch.row.iter().copied());
            if let Some(mut joined) = unified(joined, &pairs) {
                joined.row = join_row(&joined.row, width, output)?;
                branches.push(joined);
            }
        }
    }
    // A join whose every key is the file's own column states no key within the file: its rows are a product.
    let keyless = key_pairs(left_on, right_on)
        .all(|(left_key, _)| left_shape.0.get(left_key) == Some(&CellKind::File));
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
fn antijoin_arm(
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
        for (left_key, right_key) in key_pairs(left_on, right_on) {
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
    Some(Flattened {
        emptiness,
        ..lefts
    })
}

/// The made table of a fill over a definition's rows, and what it is of.
fn fill_of(state: &RuleFlattening<'_>, step: StepId, from: DefinitionId) -> Option<MadeOf> {
    let node = digest_of(rule(state)?, step);
    let home = state.store.0.get(&node).map_or(
        NodeHome {
            rule: state.place,
            step,
        },
        |stored| stored.home,
    );
    Some(MadeOf {
        node,
        home,
        emptiness: Emptiness::Definition(from),
    })
}

/// The made table of a step: made the first time the step's node is flattened, and found after, in whatever rule.
fn made_once(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    of: (StepId, DefinitionId),
    fill: MadeFill,
    width: usize,
) -> Option<MadeTableId> {
    let node = digest_of(rule(state)?, of.0);
    if let Some(table) = drafts.made.get(&node) {
        return Some(*table);
    }
    let table = made(drafts, fill_of(state, of.0, of.1)?, fill, width);
    let _new = drafts.made.insert(node, table);
    Some(table)
}

/// A list of a row's columns, as a fill names them.
fn column_list(columns: impl IntoIterator<Item = usize>) -> ColumnList {
    let made = columns
        .into_iter()
        .filter_map(|column| u16::try_from(column).ok().map(MadeColumn));
    ColumnList(made.collect())
}

/// The branch of a group-count: one row of its table, the keys then the count. Keyed on the file alone it is one row a file.
fn group_count_arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    step: StepId,
    operation: &Operation,
) -> Option<Flattened> {
    let Operation::GroupCount { from, keys, .. } = operation else {
        return None;
    };
    let shape = rule(state)?.shapes.get(usize::from(step))?.clone()?;
    let read = input(state, drafts, *from, InputWant::Defined)?;
    let Emptiness::Definition(definition) = read.emptiness else {
        return None;
    };
    let input = FillInput {
        pipelines: [scanned(drafts, definition)].into(),
        keys: column_list(keys.iter().map(|key| usize::from(*key))),
        order: None,
    };
    let fill = MadeFill::Group(GroupFill::Count { distinct: None }, input);
    let table = made_once(state, drafts, (step, definition), fill, shape.0.len())?;
    let key_kinds = shape.0.iter().take(keys.len());
    let per_file = if !keys.is_empty() && key_kinds.clone().all(|kind| *kind == CellKind::File) {
        RowsPerFile::One
    } else {
        RowsPerFile::Many
    };
    let cells = fresh_cells(drafts, &shape);
    Some(Flattened {
        branches: vec![FlatBranch {
            atoms: vec![FlatAtom::Rows {
                rows: AtomRows::Made(table),
                cells: cells.clone(),
                per_file,
            }],
            row: cells,
        }],
        emptiness: derived(operation, vec![Emptiness::Definition(definition)]),
        keyless: Vec::new(),
    })
}

/// The branches of an as-of: each left branch with the rows of the right side's ordered table nearest its order.
fn as_of_arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    step: StepId,
    operation: &Operation,
) -> Option<Flattened> {
    let Operation::AsOf {
        left,
        right,
        left_on,
        right_on,
        left_order,
        right_order,
        bound,
        output,
    } = operation
    else {
        return None;
    };
    let right_shape: StepShape = input_shape(&rule(state)?.shapes, *right)?;
    let mut lefts = input(state, drafts, *left, InputWant::Inline)?;
    let rights = input(state, drafts, *right, InputWant::Defined)?;
    let Emptiness::Definition(definition) = rights.emptiness else {
        return None;
    };
    // The file's own column is no key within the file: the group is the other keys.
    let within = |right_key: usize| right_shape.0.get(right_key) != Some(&CellKind::File);
    let keys: Vec<(usize, usize)> = key_pairs(left_on, right_on)
        .filter(|(_, right_key)| within(*right_key))
        .collect();
    let sorted = FillInput {
        pipelines: [scanned(drafts, definition)].into(),
        keys: column_list(keys.iter().map(|(_, right_key)| *right_key)),
        order: u16::try_from(usize::from(*right_order)).ok().map(MadeColumn),
    };
    let fill = MadeFill::Ordered(OrderedFill::Sorted(sorted));
    let table = made_once(state, drafts, (step, definition), fill, right_shape.0.len())?;
    for branch in &mut lefts.branches {
        let key = picked(&branch.row, keys.iter().map(|(left_key, _)| *left_key))?;
        let cells = fresh_cells(drafts, &right_shape);
        let width = branch.row.len();
        let mut row = branch.row.clone();
        row.extend(cells.iter().copied());
        branch.atoms.push(FlatAtom::Latest {
            table,
            key,
            bound: *branch.row.get(usize::from(*left_order))?,
            side: *bound,
            cells,
        });
        branch.row = join_row(&row, width, output)?;
    }
    let emptiness = derived(operation, vec![lefts.emptiness, rights.emptiness]);
    Some(Flattened {
        emptiness,
        ..lefts
    })
}

/// The flattening of one step. None for an operation the lowering does not evaluate: the rule was refused before it came here.
fn arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    step: StepId,
    operation: &Operation,
) -> Option<Flattened> {
    match operation {
        Operation::Select {
            from,
            comparisons,
            kept,
        } => {
            let mut read = input(state, drafts, *from, InputWant::Inline)?;
            read.branches = all_selected(state, drafts, read.branches, comparisons)?;
            if !kept.is_empty() {
                for branch in &mut read.branches {
                    branch.row = picked(&branch.row, kept.iter().map(|column| usize::from(*column)))?;
                }
            }
            read.emptiness = selection_emptiness(state, drafts, *from, comparisons, read.emptiness);
            Some(read)
        }
        Operation::Project { from, columns, .. } => {
            let mut read = input(state, drafts, *from, InputWant::Inline)?;
            for branch in &mut read.branches {
                branch.row = picked(&branch.row, columns.iter().map(|column| usize::from(*column)))?;
            }
            Some(read)
        }
        Operation::Extend { from, values } => {
            let mut read = input(state, drafts, *from, InputWant::Inline)?;
            for branch in &mut read.branches {
                for value in values {
                    let added = term(state, drafts, &branch.row, value)?;
                    branch.row.push(added);
                }
            }
            Some(read)
        }
        Operation::Join { .. } => join_arm(state, drafts, step, operation),
        Operation::Union { left, right } => {
            let mut lefts = input(state, drafts, *left, InputWant::Inline)?;
            let rights = input(state, drafts, *right, InputWant::Inline)?;
            lefts.branches.extend(rights.branches);
            lefts.keyless.extend(rights.keyless);
            lefts.emptiness = derived(operation, vec![lefts.emptiness, rights.emptiness]);
            Some(lefts)
        }
        Operation::Antijoin { .. } => antijoin_arm(state, drafts, operation),
        Operation::GroupCount { .. } => group_count_arm(state, drafts, step, operation),
        Operation::AsOf { .. } => as_of_arm(state, drafts, step, operation),
        // What is left to lower, each by name: `tables::service::shaped` refuses a rule that holds one.
        Operation::Axis { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::GroupDigest { .. }
        | Operation::GroupBound { .. }
        | Operation::GroupOrdinal { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::Words { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Closure { .. }
        | Operation::Follow { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Worklist { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => None,
    }
}

/// The steps of the rule that are flattened: those its emit reaches without passing a step that already has a definition.
/// Each is visited once.
fn wanted(state: &RuleFlattening<'_>, drafts: &Drafts) -> Option<Vec<bool>> {
    let rule = rule(state)?;
    let plan = rule.rule.plan;
    let mut wanted = vec![false; plan.steps.len()];
    let mut waiting = vec![plan.emit.step];
    while let Some(step) = waiting.pop() {
        let place = usize::from(step);
        if wanted.get(place) != Some(&false) || drafts.defined.contains_key(&digest_of(rule, step)) {
            continue;
        }
        *wanted.get_mut(place)? = true;
        let inputs = ryo_values::domain::plans::service::inputs_of(&plan.steps.get(place)?.operation);
        for read in ryo_values::domain::plans::service::named(&inputs) {
            if let Input::Step(read) = read {
                waiting.push(*read);
            }
        }
    }
    Some(wanted)
}

/// The rows a rule emits, flattened: every step once, in step order. The step named is one that could not be flattened.
pub(crate) fn rule_flattened(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
) -> Result<Flattened, StepId> {
    let emitted = rule(state).map_or(StepId(0), |rule| rule.rule.plan.emit.step);
    let wanted = wanted(state, drafts).ok_or(emitted)?;
    let plan = rule(state).ok_or(emitted)?.rule.plan;
    state.flat = plan.steps.iter().map(|_| None).collect();
    for (place, step) in plan.steps.iter().enumerate() {
        if wanted.get(place) != Some(&true) {
            continue;
        }
        let id = StepId(u32::try_from(place).unwrap_or(u32::MAX));
        let flattened = arm(state, drafts, id, &step.operation).ok_or(id)?;
        *state.flat.get_mut(place).ok_or(id)? = Some(flattened);
    }
    input(state, drafts, Input::Step(emitted), InputWant::Inline).ok_or(emitted)
}
