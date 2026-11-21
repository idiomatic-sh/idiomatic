//! The services of the `completing` part: one completion of a branch's order, by the cost model (`swarm/cut`). A test runs
//! as soon as its terms are given. Of the keyed accesses the lowest rank goes next: an access ranks as the rows it gives, a
//! use of a definition as the rows or the steps of the definition's own plan for the columns bound. A decision goes before
//! any access ranked above one row; a dear one waits for a cheap keyed access. An access with no key is the unit's first
//! enumeration, or a product stage of a pipeline: a branch and a pattern that would need one anywhere else have no program.

use crate::domain::lowering::costing::constants::{
    CONSTANT_EQUAL_KEEPS, DEAR_DECISION, DEARER_DECISION, DECISION_WAITS, DERIVED_NEAREST,
    EQUAL_KEEPS, KEYED_STEPS, LEAST_ROWS, ONE, TEST_KEEPS, UNEQUAL_KEEPS, USE_RANK,
};
use crate::domain::lowering::costing::service::{
    keyed, kind_extent, measured, planned_rows, plus, source_rows, times,
};
use crate::domain::lowering::costing::types::{AccessCost, CellState, PlanEstimate};
use crate::domain::lowering::flattening::service::{made_column, pins_of, variables};
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable, NearestOver, RowColumn, RowsPerFile,
};
use crate::domain::lowering::planning::operands::service::uses_of;
use crate::domain::lowering::planning::service::state_of;
use crate::domain::lowering::planning::types::{
    AtomPlace, AtomPlacing, Enumerated, GivenTerms, NextAccess, OrderCompletion, PlannedBranch,
    PlanningWith, TermGiven, UnitRole, UnitStart, WaitingAtom,
};
use crate::domain::lowering::planning::uses::service::{closure, made, read, used, walks_edges};
use crate::domain::lowering::planning::walks::service::walk_rows;
use crate::domain::steps::types::{AxisDirection, ColumnMask, CostClass, TestOperation};
use ryo_relations::domain::catalog::service::{fact_form, node_column};
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::types::{Count, FactForm, NodeColumn};
use std::collections::HashSet;

/// Whether a term is given: a constant, the file, or a variable some access gave.
fn ready(term: FlatTerm, given: &GivenTerms<'_>) -> TermGiven {
    match term {
        FlatTerm::Variable(variable) if !given.variables.contains(&variable) => TermGiven::Open,
        FlatTerm::Variable(_) | FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File => {
            TermGiven::Given
        }
    }
}

/// Whether every term is given.
fn all_ready<'t>(
    terms: impl IntoIterator<Item = &'t FlatTerm>,
    given: &GivenTerms<'_>,
) -> TermGiven {
    let mut terms = terms.into_iter();
    if terms.all(|term| ready(*term, given) == TermGiven::Given) {
        TermGiven::Given
    } else {
        TermGiven::Open
    }
}

/// What is given at a completion's point.
fn given_now<'a>(state: &'a OrderCompletion<'_>) -> GivenTerms<'a> {
    GivenTerms {
        variables: &state.given,
        pins: &state.pins,
    }
}

/// How a cell stands for a rows atom now. A pinned variable a use of a definition was keyed by, in a pipeline, is still
/// its constant: the use left no row to read the cell from, so no stage after it can take the cell in a register.
fn cell_state(state: &OrderCompletion<'_>, cell: FlatTerm) -> CellState {
    match cell {
        FlatTerm::Variable(variable) if state.rowless.contains(&variable) => CellState::Constant,
        FlatTerm::Variable(_) | FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File => {
            state_of(cell, &given_now(state))
        }
    }
}

/// What the completion is planned with.
const fn with<'a>(state: &OrderCompletion<'a>) -> PlanningWith<'a> {
    PlanningWith {
        drafts: state.drafts,
        costs: state.costs,
    }
}

/// An access that gives `rows` for each row of the unit and takes a step for each row it gives, one at least.
fn reading(rows: PlanEstimate) -> AccessCost {
    AccessCost {
        rows,
        steps: rows.max(ONE),
        once: PlanEstimate(0),
    }
}

/// The access of a rows atom now, with its class. A source of one row a file is keyed by the file once the unit has
/// enumerated, and is the unit's own first enumeration before. A use of a definition by a register costs what the
/// definition's own plan for the bound columns does; the rows of a definition read with no register are its table's, made
/// once; an access with no register after the first enumeration is every row of its source.
fn access(state: &OrderCompletion<'_>, atom: &FlatAtom) -> Option<(CostClass, AccessCost)> {
    let FlatAtom::Rows {
        rows,
        cells,
        per_file,
    } = atom
    else {
        return None;
    };
    let states: Vec<CellState> = cells.iter().map(|cell| cell_state(state, *cell)).collect();
    let mut choice = keyed(state.drafts, *rows, *per_file, &states);
    // A bucket a sample sized is taken at its measured rows, not at the fixed prior.
    let held: Vec<Option<FlatTerm>> = cells
        .iter()
        .map(|cell| match cell {
            FlatTerm::Text(_) | FlatTerm::Number(_) => Some(*cell),
            FlatTerm::Variable(variable) => state.pins.get(variable).copied(),
            FlatTerm::File => None,
        })
        .collect();
    if let Some(rows) = measured(state.drafts, *rows, &states, &held) {
        choice.rows = rows;
    }
    if *per_file == RowsPerFile::One && state.enumerated == Enumerated::NotYet {
        choice.class = CostClass::File;
    }
    let product = choice.class == CostClass::File && state.enumerated == Enumerated::Already;
    let given_rows = if product {
        planned_rows(source_rows(state.drafts, *rows))
    } else {
        choice.rows
    };
    let cost = match (*rows, choice.class) {
        (AtomRows::Definition(definition), CostClass::Fanout) => {
            let columns = choice.cells.iter().filter_map(|cell| made_column(*cell));
            let bound = ColumnMask(columns.collect());
            used(with(state), definition, &bound, choice.rows)
        }
        (AtomRows::Definition(definition), _) => read(with(state), definition, given_rows),
        (AtomRows::Made(table), _) => made(state.drafts, table, given_rows),
        (AtomRows::Relation(_), _) => reading(given_rows),
    };
    // An access by a key whose other cells nothing else reads only asks whether a row is there: one row at most. (The
    // emitter writes it so: `emitting::rows_read`, `used`.) A pinned variable is read by its test.
    let unread = |cell: &FlatTerm, standing: &CellState| match (cell, standing) {
        (FlatTerm::Variable(variable), CellState::Open) => {
            state.uses.get(variable).is_none_or(|uses| uses.0 < 2)
        }
        (FlatTerm::Variable(_), CellState::Constant) => false,
        (
            FlatTerm::Variable(_) | FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File,
            CellState::Register | CellState::Constant | CellState::File | CellState::Open,
        ) => true,
    };
    let asked_present = choice.class == CostClass::Fanout
        && state.enumerated == Enumerated::Already
        && cells
            .iter()
            .zip(&states)
            .all(|(cell, standing)| unread(cell, standing));
    let cost = if asked_present {
        AccessCost {
            rows: cost.rows.min(ONE),
            ..cost
        }
    } else {
        cost
    };
    Some((choice.class, cost))
}

/// The rank of an access: the rows it gives; for a use of a definition its steps over those of a well-keyed call when
/// that is more, and never under a use's least rank.
fn rank(atom: &FlatAtom, class: CostClass, cost: AccessCost) -> PlanEstimate {
    match (atom, class) {
        (
            FlatAtom::Rows {
                rows: AtomRows::Definition(_),
                ..
            },
            CostClass::Fanout,
        ) => cost
            .rows
            .max(PlanEstimate(cost.steps.0 / KEYED_STEPS))
            .max(USE_RANK),
        // A climb over derived edges ranks as its steps: a test of what it starts from goes before it.
        (
            FlatAtom::Nearest {
                over: NearestOver::Derived(_),
                ..
            },
            _,
        ) => cost.rows.max(cost.steps),
        (
            FlatAtom::Rows { .. }
            | FlatAtom::Latest { .. }
            | FlatAtom::Decided { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Closure { .. }
            | FlatAtom::Called { .. }
            | FlatAtom::Test { .. },
            _,
        ) => cost.rows,
    }
}

/// The share of its rows a test keeps: a tenth for an equality with a constant, three tenths for an equality of two
/// cells, nearly all for an inequality, seven tenths for any other. A pin an access was keyed by holds already.
fn kept(state: &OrderCompletion<'_>, test: TestOperation, terms: [&FlatTerm; 2]) -> PlanEstimate {
    let constant = terms
        .iter()
        .any(|term| matches!(term, FlatTerm::Text(_) | FlatTerm::Number(_)));
    let implied = terms.iter().any(|term| match term {
        FlatTerm::Variable(variable) => state.implied.contains(variable),
        FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File => false,
    });
    match test {
        TestOperation::Equal if implied && constant => ONE,
        TestOperation::Equal | TestOperation::In if constant => CONSTANT_EQUAL_KEEPS,
        TestOperation::Equal | TestOperation::In => EQUAL_KEEPS,
        TestOperation::NotEqual | TestOperation::NotIn => UNEQUAL_KEEPS,
        TestOperation::Less
        | TestOperation::LessOrEqual
        | TestOperation::Greater
        | TestOperation::GreaterOrEqual
        | TestOperation::Matches(_)
        | TestOperation::Glob
        | TestOperation::NotGlob
        | TestOperation::StartsWith
        | TestOperation::EndsWith
        | TestOperation::NumericAtLeast
        | TestOperation::NaturalLess
        | TestOperation::Successor => TEST_KEEPS,
    }
}

/// The pinned variables among the cells a rows atom is keyed by now: the test that pins each holds once the atom is
/// placed. With them, whether the access leaves no row a pipeline could read such a cell from: a use of a definition by a
/// register gives none, so a pin it is keyed by stays a constant for the stages after it.
fn keyed_pins(state: &OrderCompletion<'_>, atom: &FlatAtom) -> (Vec<FlatVariable>, bool) {
    let FlatAtom::Rows {
        rows,
        cells,
        per_file,
    } = atom
    else {
        return (Vec::new(), false);
    };
    let states: Vec<CellState> = cells.iter().map(|cell| cell_state(state, *cell)).collect();
    let choice = keyed(state.drafts, *rows, *per_file, &states);
    // After the first enumeration an access with no register is every row of its source: no pin keys it.
    if choice.class == CostClass::File && state.enumerated == Enumerated::Already {
        return (Vec::new(), false);
    }
    let keys = choice.cells.iter();
    let mut pins: Vec<FlatVariable> = keys
        .filter(|cell| states.get(cell.0) == Some(&CellState::Constant))
        .filter_map(|cell| match cells.get(cell.0) {
            Some(FlatTerm::Variable(variable)) => Some(*variable),
            Some(FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File) | None => None,
        })
        .collect();
    // A parent's children are sized in the field they are held to (`costing::node_access`): the test that pins the field
    // holds in that size already, and keeps no further share of them.
    if let AtomRows::Relation(relation) = rows
        && fact_form(*relation) == FactForm::NodeTable
        && choice.class == CostClass::Fanout
    {
        let listed = columns(*relation);
        let named = |place: usize, wanted: NodeColumn| {
            listed.get(place).copied().and_then(node_column) == Some(wanted)
        };
        if choice
            .cells
            .iter()
            .any(|cell| named(cell.0, NodeColumn::Parent))
        {
            let fields = cells.iter().enumerate().filter_map(|(place, cell)| {
                let held = states.get(place) == Some(&CellState::Constant);
                match cell {
                    FlatTerm::Variable(variable) if held && named(place, NodeColumn::Field) => {
                        Some(*variable)
                    }
                    FlatTerm::Variable(_)
                    | FlatTerm::Text(_)
                    | FlatTerm::Number(_)
                    | FlatTerm::File => None,
                }
            });
            pins.extend(fields);
        }
    }
    let rowless = matches!(rows, AtomRows::Definition(_))
        && choice.class == CostClass::Fanout
        && state.role == UnitRole::Pipeline;
    (pins, rowless)
}

/// Places an atom: its variables are given from here on, its steps are taken for each row the unit holds, and the unit
/// holds the rows it gives. What it takes once a file is taken in the share of files a row gets here.
fn place(state: &mut OrderCompletion<'_>, next: NextAccess) {
    let Some(atom) = state.branch.atoms.get(next.atom.0) else {
        return;
    };
    let (pins, rowless) = keyed_pins(state, atom);
    if let Some(placed) = state.placed.get_mut(next.atom.0) {
        *placed = AtomPlacing::Placed;
    }
    state.order.push(next.atom);
    // A table is made once a file, in the share of files a row gets here: the dearest one the order reads is kept.
    state.tables = state.tables.max(times(next.cost.once, state.rows.min(ONE)));
    let taken = times(state.rows, next.cost.steps);
    match atom {
        FlatAtom::Rows { .. }
        | FlatAtom::Latest { .. }
        | FlatAtom::Walk { .. }
        | FlatAtom::Nearest { .. }
        | FlatAtom::Closure { .. } => {
            state.given.extend(variables(atom));
            if rowless {
                state.rowless.extend(pins.iter().copied());
            }
            state.implied.extend(pins);
            state.enumerated = Enumerated::Already;
            state.cost = plus(state.cost, taken);
            state.rows = times(state.rows, next.cost.rows.max(LEAST_ROWS));
        }
        // A per-row meaning gives its outputs, and a decision keeps or drops a row: the rows stay as they are.
        FlatAtom::Called { .. } => {
            state.given.extend(variables(atom));
            state.cost = plus(state.cost, taken);
        }
        FlatAtom::Decided { .. } => state.cost = plus(state.cost, taken),
        FlatAtom::Test { test, left, right } => {
            state.rows = times(state.rows, kept(state, *test, [left, right]));
        }
    }
}

/// The unplaced atoms, by place.
fn waiting<'a>(state: &'a OrderCompletion<'_>) -> impl Iterator<Item = WaitingAtom<'a>> {
    let atoms = state.branch.atoms.iter().enumerate();
    atoms
        .filter(|(at, _)| state.placed.get(*at) == Some(&AtomPlacing::Waiting))
        .map(|(at, atom)| WaitingAtom {
            place: AtomPlace(at),
            atom,
        })
}

/// Whether the unit may run a test or a decision now: a branch and a pipeline run none before their first enumeration,
/// which is their first step or stage.
fn settled(state: &OrderCompletion<'_>) -> TermGiven {
    if state.role != UnitRole::Pattern && state.enumerated == Enumerated::NotYet {
        TermGiven::Open
    } else {
        TermGiven::Given
    }
}

/// Whether a test of these terms can be written now. A program holds the file; a pipeline reads the file's cell from a
/// row at hand that holds it: a relation's row, or the definition's row its scan starts from (a keyed use gives no row).
fn file_held(state: &OrderCompletion<'_>, terms: [&FlatTerm; 2]) -> TermGiven {
    let asked = terms.iter().any(|term| **term == FlatTerm::File);
    let scanned = state.order.first().copied();
    let mut atoms = state.branch.atoms.iter().enumerate().zip(&state.placed);
    let held = atoms.any(|((place, atom), placing)| match (placing, atom) {
        (AtomPlacing::Placed, FlatAtom::Rows { rows, cells, .. }) => {
            cells.contains(&FlatTerm::File)
                && match rows {
                    AtomRows::Relation(_) => true,
                    AtomRows::Definition(_) => scanned == Some(AtomPlace(place)),
                    AtomRows::Made(_) => false,
                }
        }
        (AtomPlacing::Waiting, FlatAtom::Rows { .. })
        | (
            AtomPlacing::Placed | AtomPlacing::Waiting,
            FlatAtom::Latest { .. }
            | FlatAtom::Decided { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Closure { .. }
            | FlatAtom::Called { .. }
            | FlatAtom::Test { .. },
        ) => false,
    });
    if asked && state.role == UnitRole::Pipeline && !held {
        TermGiven::Open
    } else {
        TermGiven::Given
    }
}

/// The first waiting test whose terms are given: a walk between two given elements is a test of them.
fn test(state: &OrderCompletion<'_>) -> Option<NextAccess> {
    let given = given_now(state);
    let found = waiting(state).find(|waiting| match waiting.atom {
        FlatAtom::Test { left, right, .. }
        | FlatAtom::Walk {
            start: left,
            reached: right,
            ..
        } => {
            all_ready([left, right], &given) == settled(state)
                && file_held(state, [left, right]) == TermGiven::Given
        }
        // A per-row meaning runs as soon as what it reads is given.
        FlatAtom::Called { arguments, .. } => all_ready(arguments, &given) == settled(state),
        FlatAtom::Rows { .. }
        | FlatAtom::Latest { .. }
        | FlatAtom::Nearest { .. }
        | FlatAtom::Closure { .. }
        | FlatAtom::Decided { .. } => false,
    });
    found
        .filter(|_| settled(state) == TermGiven::Given)
        .map(|waiting| NextAccess {
            atom: waiting.place,
            cost: reading(ONE),
            rank: ONE,
        })
}

/// What a decision costs for each row: a definition asked by the cells given runs the program its own plan has for
/// those columns, or reads its table; a relation's own rows are asked in place.
fn decision_cost(state: &OrderCompletion<'_>, atom: &FlatAtom) -> Option<AccessCost> {
    let FlatAtom::Decided { rows, cells, .. } = atom else {
        return None;
    };
    let given = given_now(state);
    if all_ready(cells.iter().flatten(), &given) == TermGiven::Open {
        return None;
    }
    let AtomRows::Definition(definition) = rows else {
        return Some(reading(ONE));
    };
    let asked = cells.iter().enumerate().filter_map(|(place, cell)| {
        let bound = matches!(
            cell,
            Some(FlatTerm::Variable(_) | FlatTerm::Text(_) | FlatTerm::Number(_))
        );
        bound.then(|| made_column(RowColumn(place))).flatten()
    });
    let bound = ColumnMask(asked.collect());
    let cost = if bound.0.is_empty() {
        read(with(state), *definition, ONE)
    } else {
        used(with(state), *definition, &bound, ONE)
    };
    Some(AccessCost {
        rows: ONE,
        steps: plus(cost.steps, ONE),
        once: cost.once,
    })
}

/// The cheapest waiting decision whose asked cells are given.
fn decision(state: &OrderCompletion<'_>) -> Option<NextAccess> {
    if settled(state) == TermGiven::Open {
        return None;
    }
    waiting(state)
        .filter_map(|waiting| {
            decision_cost(state, waiting.atom).map(|cost| NextAccess {
                atom: waiting.place,
                cost,
                rank: cost.steps,
            })
        })
        .min_by_key(|next| (next.rank, next.atom))
}

/// The rank a decision waits for: an access ranked up to it goes first. Any decision waits for a read of one row by its
/// key; a dear one for a cheap keyed access too, and a dearer one for more.
fn waited(decision: NextAccess) -> PlanEstimate {
    if decision.cost.steps >= DEARER_DECISION.0 {
        DEARER_DECISION.1
    } else if decision.cost.steps >= DEAR_DECISION.0 {
        DEAR_DECISION.1
    } else {
        DECISION_WAITS
    }
}

/// The kind a branch holds an element to: the kind cell of a node relation's row of that element, when it is a constant
/// or pinned to one.
fn kind_held(state: &OrderCompletion<'_>, element: FlatTerm) -> Option<FlatTerm> {
    state.branch.atoms.iter().find_map(|atom| {
        let FlatAtom::Rows {
            rows: AtomRows::Relation(relation),
            cells,
            ..
        } = atom
        else {
            return None;
        };
        let listed = columns(*relation);
        let at = |wanted: NodeColumn| {
            let mut places = listed.iter().zip(cells);
            places
                .find_map(|(column, cell)| (node_column(*column) == Some(wanted)).then_some(*cell))
        };
        if fact_form(*relation) != FactForm::NodeTable || at(NodeColumn::Id) != Some(element) {
            return None;
        }
        match at(NodeColumn::Kind)? {
            FlatTerm::Variable(variable) => state.pins.get(&variable).copied(),
            constant @ (FlatTerm::Text(_) | FlatTerm::Number(_)) => Some(constant),
            FlatTerm::File => None,
        }
    })
}

/// The rows a walk gives from its given end: the fixed prior, and, down the tree from an element of a kind a sample
/// sized, what lies under one element of that kind when that is more: an element of a rare kind holds much of its file.
fn walked_rows(
    state: &OrderCompletion<'_>,
    direction: AxisDirection,
    start: FlatTerm,
    reached: FlatTerm,
) -> Option<PlanEstimate> {
    let given = given_now(state);
    let (from, to) = (ready(start, &given), ready(reached, &given));
    let rows = walk_rows(direction, from, to)?;
    let top = match (direction, from, to) {
        (AxisDirection::Descendant, TermGiven::Given, TermGiven::Open) => Some(start),
        (AxisDirection::Ancestor, TermGiven::Open, TermGiven::Given) => Some(reached),
        (
            AxisDirection::Parent
            | AxisDirection::Ancestor
            | AxisDirection::Descendant
            | AxisDirection::NextSibling
            | AxisDirection::PreviousSibling,
            TermGiven::Given | TermGiven::Open,
            TermGiven::Given | TermGiven::Open,
        ) => None,
    };
    let under = top
        .and_then(|top| kind_held(state, top))
        .and_then(|kind| kind_extent(state.drafts, kind));
    Some(under.map_or(rows, |under| rows.max(under)))
}

/// The lowest-ranked waiting access of the wanted class: keyed (a fan-out), or with no key (the file's rows).
fn cheapest(state: &OrderCompletion<'_>, class: CostClass) -> Option<NextAccess> {
    let given = given_now(state);
    let keyed = |ready: TermGiven| ready == TermGiven::Given && class == CostClass::Fanout;
    waiting(state)
        .filter_map(|waiting| {
            let (atom, each) = (waiting.place, waiting.atom);
            let cost = match each {
                FlatAtom::Rows { .. } => access(state, each)
                    .filter(|(found, _)| *found == class)
                    .map(|(_, cost)| cost),
                FlatAtom::Latest { key, bound, .. } => {
                    keyed(all_ready(key.iter().chain([bound]), &given)).then_some(reading(ONE))
                }
                FlatAtom::Walk {
                    direction,
                    start,
                    reached,
                    ..
                } => walked_rows(state, *direction, *start, *reached)
                    .filter(|_| class == CostClass::Fanout)
                    .map(reading),
                FlatAtom::Nearest { over, start, .. } => {
                    keyed(ready(*start, &given)).then_some(match over {
                        NearestOver::Tree(_) => reading(ONE),
                        NearestOver::Derived(_) => AccessCost {
                            rows: ONE,
                            steps: DERIVED_NEAREST,
                            once: PlanEstimate(0),
                        },
                    })
                }
                // A closure walks from its source, or back from what is reached, within keys that are given.
                FlatAtom::Closure {
                    edges,
                    keys,
                    source,
                    reached,
                } => {
                    let from_source = ready(*source, &given) == TermGiven::Given;
                    let to_reached = ready(*reached, &given) == TermGiven::Given;
                    let entered = from_source || to_reached;
                    (entered && keyed(all_ready(keys, &given))).then(|| {
                        let walk = closure(with(state), edges, from_source);
                        // Between two given elements the walk is a test of them: one row at most.
                        let rows = if from_source && to_reached {
                            walk.rows.min(ONE)
                        } else {
                            walk.rows
                        };
                        AccessCost { rows, ..walk }
                    })
                }
                FlatAtom::Decided { .. } | FlatAtom::Called { .. } | FlatAtom::Test { .. } => None,
            };
            cost.map(|cost| NextAccess {
                atom,
                cost,
                rank: rank(each, class, cost),
            })
        })
        .min_by_key(|next| (next.rank, next.atom))
}

/// The access each rows atom of a branch is as a unit's first enumeration, nothing enumerated yet: its anchors.
pub(crate) fn anchors(
    with: PlanningWith<'_>,
    branch: &FlatBranch,
    start: UnitStart<'_>,
    role: UnitRole,
) -> Vec<NextAccess> {
    let state = begun(with, branch, start, role);
    let atoms = branch.atoms.iter().enumerate();
    let found: Vec<(bool, NextAccess)> = atoms
        .filter_map(|(at, atom)| {
            access(&state, atom).map(|(class, cost)| {
                // Read whole, a definition whose plan walks a closure is every pair the walk reaches in the file.
                let unbounded = match atom {
                    FlatAtom::Rows {
                        rows: AtomRows::Definition(definition),
                        ..
                    } => class != CostClass::Fanout && walks_edges(with.drafts, *definition, 0),
                    FlatAtom::Rows { .. }
                    | FlatAtom::Latest { .. }
                    | FlatAtom::Decided { .. }
                    | FlatAtom::Walk { .. }
                    | FlatAtom::Nearest { .. }
                    | FlatAtom::Closure { .. }
                    | FlatAtom::Called { .. }
                    | FlatAtom::Test { .. } => false,
                };
                let anchor = NextAccess {
                    atom: AtomPlace(at),
                    cost,
                    rank: rank(atom, class, cost),
                };
                (unbounded, anchor)
            })
        })
        .collect();
    // Such a read is an anchor only when the branch has no other.
    let bounded = found.iter().any(|(unbounded, _)| !unbounded);
    found
        .into_iter()
        .filter(|(unbounded, _)| !(bounded && *unbounded))
        .map(|(_, anchor)| anchor)
        .collect()
}

/// A completion with nothing placed.
fn begun<'a>(
    with: PlanningWith<'a>,
    branch: &'a FlatBranch,
    start: UnitStart<'_>,
    role: UnitRole,
) -> OrderCompletion<'a> {
    OrderCompletion {
        drafts: with.drafts,
        costs: with.costs,
        branch,
        role,
        given: start.given.clone(),
        pins: pins_of(branch),
        placed: vec![AtomPlacing::Waiting; branch.atoms.len()],
        order: Vec::with_capacity(branch.atoms.len()),
        uses: uses_of(branch, None),
        implied: HashSet::new(),
        rowless: HashSet::new(),
        tables: PlanEstimate(0),
        rows: start.rows,
        cost: PlanEstimate(0),
        enumerated: Enumerated::NotYet,
        products: Count(0),
    }
}

/// One completion of a branch's order from `first`, when given: none when the role forbids an access the branch needs.
pub(crate) fn completed(
    with: PlanningWith<'_>,
    branch: &FlatBranch,
    start: UnitStart<'_>,
    role: UnitRole,
    first: Option<NextAccess>,
) -> Option<PlannedBranch> {
    let mut state = begun(with, branch, start, role);
    if let Some(first) = first {
        place(&mut state, first);
    }
    while state.order.len() < branch.atoms.len() {
        if let Some(next) = test(&state) {
            place(&mut state, next);
            continue;
        }
        let keyed = cheapest(&state, CostClass::Fanout);
        let next = match (decision(&state), keyed) {
            (Some(decision), Some(keyed)) if keyed.rank > waited(decision) => Some(decision),
            (Some(decision), None) => Some(decision),
            (Some(_) | None, keyed) => keyed,
        };
        if let Some(next) = next {
            place(&mut state, next);
            continue;
        }
        // No access is keyed: the unit's first enumeration, or a product stage of a pipeline; else there is no program.
        let next = cheapest(&state, CostClass::File)?;
        match (role, state.enumerated) {
            (UnitRole::Branch | UnitRole::Pipeline, Enumerated::NotYet) => place(&mut state, next),
            (UnitRole::Pipeline, Enumerated::Already) => {
                state.products = Count(state.products.0.saturating_add(1));
                place(&mut state, next);
            }
            (UnitRole::Pattern, _) | (UnitRole::Branch, Enumerated::Already) => return None,
        }
    }
    Some(PlannedBranch {
        order: state.order,
        cost: state.cost,
        tables: state.tables,
        rows: state.rows,
        products: state.products,
    })
}
