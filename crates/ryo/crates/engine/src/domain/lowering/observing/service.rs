//! The services of the `observing` part: whose rows a step holds (`local_of`), the shape and the branches of a relation the
//! host declares and of rows a query wrote, the producers of each bound observation resolved against the accepted rules,
//! and the one definition a bound observation is read as. Every operation is named in `local_of`: a new one is no step of
//! the file after the files until it says so here.

use super::types::{Local, ResolvedBinding};
use crate::domain::lowered::types::Emptiness;
use crate::domain::lowering::definitions::service::{defined, regex, text};
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::flattening::types::{FlatAtom, FlatBranch, FlatTerm};
use crate::domain::lowering::nodes::service::{digest_of, input_shape, reached_from};
use crate::domain::lowering::nodes::types::{AdmittedRule, NodeHome, StepShape};
use crate::domain::lowering::tables::inputs::service::{rule, use_of};
use crate::domain::lowering::tables::service::shape_reached;
use crate::domain::lowering::tables::shapes::service::viewed;
use crate::domain::lowering::tables::types::RuleFlattening;
use crate::domain::lowering::types::{BoundProducer, LoadGiven, SelectedRule};
use crate::domain::steps::types::TestOperation;
use ryo_relations::domain::relations::types::{CellKind, Count};
use ryo_values::domain::hosted::service::kind_of;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, HostedNumber, Input, LiteralText, ObservationBindingId, StepScope,
};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::row_texts::types::Cell;
use ryo_values::domain::written_rows::types::RowsSource;

/// The kinds of a bound observation's rows, as the plan declares the binding.
fn bound_shape(rule: &SelectedRule<'_>, binding: ObservationBindingId) -> Option<StepShape> {
    let place = usize::try_from(binding.0).ok()?;
    let bound = rule.plan.observation_inputs.get(place)?;
    Some(StepShape(bound.columns.iter().map(|column| column.kind).collect()))
}

/// The shape of an input: a relation's and a step's as `input_shape` has them, a bound observation's as its binding declares.
#[must_use]
pub(crate) fn input_kinds(
    rule: &SelectedRule<'_>,
    shapes: &[Option<StepShape>],
    input: Input,
) -> Option<StepShape> {
    match input {
        Input::BoundObservation(binding) => bound_shape(rule, binding),
        Input::Relation(_) | Input::Step(_) => input_shape(shapes, input),
    }
}

/// The shape of a relation the host declares: its columns' kinds. None for a relation the host does not declare.
#[must_use]
pub(crate) fn hosted_shape(given: LoadGiven<'_>, relation: HostedNumber) -> Option<StepShape> {
    let relation = given.hosted.get(usize::try_from(relation.0).ok()?)?;
    let kinds = relation.columns.iter().map(|column| kind_of(column.kind));
    Some(StepShape(kinds.collect()))
}

/// The shape of a `rows` step: a text column for each key, every written row as wide. None for the rows a parameter holds.
#[must_use]
pub(crate) fn rows_shape(operation: &Operation) -> Option<StepShape> {
    let Operation::Rows { source, keys } = operation else {
        return None;
    };
    match source {
        RowsSource::Written(rows) => rows
            .0
            .iter()
            .all(|row| row.0.len() == keys.len())
            .then(|| StepShape(vec![CellKind::Symbol; keys.len()])),
        RowsSource::Parameter(_) => None,
    }
}

/// Whether a relation the host declares names a file in a column: its rows are then rows of the files they name.
fn names_files(given: LoadGiven<'_>, relation: HostedNumber) -> bool {
    hosted_shape(given, relation).is_some_and(|shape| shape.0.contains(&CellKind::File))
}

/// Whether a column names the file its row is of: the file itself, or a node, whose id holds its file.
fn names_file(kind: Option<CellKind>) -> bool {
    matches!(kind, Some(CellKind::File | CellKind::Node))
}

/// Whether some pair of the keys holds the file on both sides (the file, or a node of it): rows of two files never meet.
fn file_keyed(
    left: &StepShape,
    right: &StepShape,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
) -> bool {
    let kind = |shape: &StepShape, column: &ColumnIndex| shape.0.get(usize::from(*column)).copied();
    left_on.iter().zip(right_on).any(|(left_key, right_key)| {
        let (left_kind, right_kind) = (kind(left, left_key), kind(right, right_key));
        left_kind == right_kind && names_file(left_kind)
    })
}

/// Whose rows two inputs meet in: both constant, a constant table; one the file's, the file's; both the file's, the file's
/// only when the step keys them by the file. `sided` says whether only the left side's rows make the answer (an absence).
fn met(left: Local, right: Local, keyed: bool, sided: bool) -> Option<Local> {
    match (left, right) {
        (Local::Constant, Local::Constant) => Some(Local::Constant),
        (Local::File, Local::Constant) => Some(Local::File),
        (Local::Constant, Local::File) => (!sided).then_some(Local::File),
        (Local::File, Local::File) => keyed.then_some(Local::File),
    }
}

/// Whose rows a step holds, from its inputs': none for a step that holds rows of several files at once, which only a pass
/// after the files can answer. A step of the file reads the file's relations: its rows are the file's unless every input is
/// a constant table. A step after the files reads no relation; it is a step of the file when the union over the files of
/// its rows file by file is its rows over the run: a row-wise operation of one input, a union of two inputs of one kind,
/// a join or an absence of the file's rows against a constant table or against the file's rows keyed by the file, a group
/// keyed by the file (a key pair that holds the file, or a node of it, on both sides). A constant table never makes a step
/// one of several files by itself. A relation the host declares is a constant table, or the file's rows when it names
/// files; a bound observation holds the rows its producers' observed steps hand over in the file.
#[must_use]
pub(crate) fn local_of(
    rule: &SelectedRule<'_>,
    given: LoadGiven<'_>,
    shapes: &[Option<StepShape>],
    locals: &[Option<Local>],
    step: &PlanStep,
) -> Option<Local> {
    let per_file = step.scope == StepScope::PerFile;
    let of = |input: &Input| match input {
        Input::Relation(_) => per_file.then_some(Local::File),
        Input::Step(read) => locals.get(usize::from(*read)).copied().flatten(),
        Input::BoundObservation(_) => Some(Local::File),
    };
    let kinds = |input: &Input| input_kinds(rule, shapes, *input);
    match &step.operation {
        Operation::Hosted { relation } => Some(if names_files(given, *relation) {
            Local::File
        } else {
            Local::Constant
        }),
        Operation::Rows { .. } => Some(Local::Constant),
        operation if per_file => {
            let inputs = inputs_of(operation);
            let constant = named(&inputs)
                .iter()
                .all(|input| of(input) == Some(Local::Constant));
            Some(if constant { Local::Constant } else { Local::File })
        }
        Operation::Select { from, .. }
        | Operation::Project { from, .. }
        | Operation::Extend { from, .. }
        | Operation::Capture { from, .. }
        | Operation::Replace { from, .. }
        | Operation::Words { from, .. }
        | Operation::SpanBetween { from, .. }
        | Operation::SpanBounds { from, .. }
        | Operation::SourceMatches { from, .. }
        | Operation::RenderSyntax { from, .. }
        | Operation::FitsRendered { from, .. } => of(from),
        Operation::GroupCount { from, keys, .. }
        | Operation::GroupDigest { from, keys, .. }
        | Operation::GroupBound { from, keys, .. }
        | Operation::GroupOrdinal { from, keys, .. } => match of(from)? {
            Local::Constant => Some(Local::Constant),
            Local::File => {
                let read = kinds(from)?;
                let by_file = keys
                    .iter()
                    .any(|key| names_file(read.0.get(usize::from(*key)).copied()));
                by_file.then_some(Local::File)
            }
        },
        Operation::Union { left, right } => {
            let (left, right) = (of(left)?, of(right)?);
            (left == right).then_some(left)
        }
        Operation::Join {
            left,
            right,
            left_on,
            right_on,
            view,
            reach: None,
            ..
        } => {
            let seen = viewed(&kinds(right)?, view)?;
            let keyed = file_keyed(&kinds(left)?, &seen, left_on, right_on);
            met(of(left)?, of(right)?, keyed, false)
        }
        Operation::AsOf {
            left,
            right,
            left_on,
            right_on,
            ..
        }
        | Operation::OrderedMatch {
            left,
            right,
            left_on,
            right_on,
            ..
        } => {
            let keyed = file_keyed(&kinds(left)?, &kinds(right)?, left_on, right_on);
            met(of(left)?, of(right)?, keyed, false)
        }
        Operation::Antijoin {
            left,
            right,
            left_on,
            right_on,
        } => {
            let keyed = file_keyed(&kinds(left)?, &kinds(right)?, left_on, right_on);
            met(of(left)?, of(right)?, keyed, true)
        }
        // A step that walks, closes or ranks rows of several files, and a join past a reach, waits for a pass after the
        // files.
        Operation::Join { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Rank { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::Follow { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Worklist { .. }
        | Operation::Cycles { .. } => None,
    }
}

/// The steps of every producer each accepted rule's bound observations read, shaped as steps of the file: a producer the
/// load did not accept, or whose observed step is no step of the file or holds other kinds than the binding declares,
/// hands over no rows. Each producer's observed step is named on it (`observed`), each rule's producers on it (`bound`).
pub(crate) fn resolved(accepted: &mut [AdmittedRule<'_>]) {
    for consumer in 0..accepted.len() {
        let Some(reader) = accepted.get(consumer) else {
            continue;
        };
        let ordinal = reader.rule.rule;
        let plan = reader.rule.plan;
        let reads = |binding: ObservationBindingId| {
            plan.steps.iter().any(|step| {
                named(&inputs_of(&step.operation)).contains(&Input::BoundObservation(binding))
            })
        };
        let wanted: Vec<_> = reader
            .given
            .observed
            .iter()
            .filter(|bound| bound.consumer == ordinal && reads(bound.binding))
            .cloned()
            .collect();
        let mut bound = Vec::with_capacity(wanted.len());
        for binding in wanted {
            let Some(declared) = accepted
                .get(consumer)
                .and_then(|reader| bound_shape(&reader.rule, binding.binding))
            else {
                continue;
            };
            let producers = binding
                .producers
                .iter()
                .filter(|producer| observed_shaped(accepted, **producer, &declared))
                .copied()
                .collect();
            bound.push(ResolvedBinding {
                binding: binding.binding,
                node: binding.node,
                producers,
            });
        }
        if let Some(reader) = accepted.get_mut(consumer) {
            reader.bound = bound;
        }
    }
}

/// Whether the observed step of `producer` is shaped as a step of the file with the kinds the binding declares: its steps
/// are then shaped, and the step named on the producer. Nothing of the producer changes when it is not.
fn observed_shaped(
    accepted: &mut [AdmittedRule<'_>],
    producer: BoundProducer,
    declared: &StepShape,
) -> bool {
    let Some(held) = accepted
        .iter_mut()
        .find(|rule| rule.rule.rule == producer.rule)
    else {
        if std::env::var_os("RYO_L2_REFUSED").is_some() {
            eprintln!("ryo l2: producer {} is not accepted: it hands no rows", producer.rule.0);
        }
        return false;
    };
    let place = usize::from(producer.step);
    let Ok(reach) = reached_from(held.rule.plan, &[producer.step]) else {
        return false;
    };
    let mut trial = held.clone();
    let reached = shape_reached(&mut trial, &reach);
    let shaped = reached.is_ok()
        && trial.locals.get(place) == Some(&Some(Local::File))
        && trial.shapes.get(place).and_then(Option::as_ref) == Some(declared);
    // Spike (ryo-fmx3.1): a producer that hands no rows over, and why, when asked.
    if !shaped && std::env::var_os("RYO_L2_REFUSED").is_some() {
        eprintln!(
            "ryo l2: producer {} step {} hands no rows: {:?}, local {:?}, shape {:?} against {:?}",
            producer.rule.0,
            producer.step.0,
            reached.err().map(|refusal| refusal.to_string()),
            trial.locals.get(place),
            trial.shapes.get(place),
            declared
        );
    }
    if shaped {
        held.shapes = trial.shapes;
        held.locals = trial.locals;
        if !held.observed.contains(&producer.step) {
            held.observed.push(producer.step);
        }
    }
    shaped
}

/// A regular expression that matches the text alone.
fn exactly(written: &str) -> LiteralText {
    let mut pattern = String::with_capacity(written.len().saturating_add(2));
    pattern.push('^');
    for character in written.chars() {
        if !character.is_alphanumeric() && character != '_' && character != '/' {
            pattern.push('\\');
        }
        pattern.push(character);
    }
    pattern.push('$');
    LiteralText(pattern.into())
}

/// The branches of a relation the host declares or of rows a query wrote: a branch for each row, its cells constants. A
/// cell that names a file is the file being scanned, and its row is a row of that file alone: the branch tests the file's
/// path. A row with a span is no row (no column the host declares holds one). None for any other operation.
pub(crate) fn constant_arm(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    operation: &Operation,
) -> Option<Flattened> {
    let mut branches = Vec::new();
    match operation {
        Operation::Hosted { relation } => {
            let given = rule(state)?.given;
            let relation = given.hosted.get(usize::try_from(relation.0).ok()?)?;
            'rows: for row in &relation.rows {
                let mut branch = FlatBranch::default();
                for cell in &row.0 {
                    let term = match cell {
                        Cell::File(path) => {
                            let pattern = exactly(path);
                            let test = TestOperation::Matches(regex(drafts, &pattern));
                            let right = FlatTerm::Text(text(drafts, state.kinds, &pattern));
                            branch.atoms.push(FlatAtom::Test {
                                test,
                                left: FlatTerm::File,
                                right,
                            });
                            FlatTerm::File
                        }
                        Cell::Text(written) => FlatTerm::Text(text(
                            drafts,
                            state.kinds,
                            &LiteralText(written.clone()),
                        )),
                        Cell::Number(number) => FlatTerm::Number(*number),
                        Cell::Span(_) => continue 'rows,
                    };
                    branch.row.push(term);
                }
                branches.push(branch);
            }
        }
        Operation::Rows {
            source: RowsSource::Written(rows),
            ..
        } => {
            for row in &rows.0 {
                let cells = row.0.iter().map(|cell| FlatTerm::Text(text(drafts, state.kinds, cell)));
                branches.push(FlatBranch {
                    atoms: Vec::new(),
                    row: cells.collect(),
                });
            }
        }
        _ => return None,
    }
    let emptiness = if branches.is_empty() {
        Emptiness::Any(Box::default())
    } else if branches.iter().all(|branch| branch.atoms.is_empty()) {
        Emptiness::All(Box::default())
    } else {
        Emptiness::Unknown
    };
    Some(Flattened {
        branches,
        emptiness,
        keyless: Vec::new(),
    })
}

/// What a step reads of a bound observation: one use of the observation's definition, whose branches are a use each of its
/// producers' observed steps' definitions (each producer was flattened first and its observed step defined). An
/// observation no producer hands rows to holds none. None when a producer's step has no definition.
pub(crate) fn bound_input(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    binding: ObservationBindingId,
) -> Option<Flattened> {
    let reader = rule(state)?;
    let width = Count(bound_shape(&reader.rule, binding)?.0.len());
    let empty = Flattened {
        branches: Vec::new(),
        emptiness: Emptiness::Any(Box::default()),
        keyless: Vec::new(),
    };
    let Some(bound) = reader.bound.iter().find(|bound| bound.binding == binding) else {
        return Some(empty);
    };
    if bound.producers.is_empty() {
        return Some(empty);
    }
    if let Some(definition) = drafts.defined.get(&bound.node).copied() {
        return Some(use_of(drafts, definition, width));
    }
    let mut branches = Vec::with_capacity(bound.producers.len());
    let mut each = Vec::with_capacity(bound.producers.len());
    for producer in &bound.producers {
        // A producer a build refused since is no rule of the pack: it hands over no rows.
        let Some(held) = state.rules.iter().find(|rule| rule.rule.rule == producer.rule) else {
            continue;
        };
        let definition = *drafts.defined.get(&digest_of(held, producer.step))?;
        branches.extend(use_of(drafts, definition, width).branches);
        each.push(Emptiness::Definition(definition));
    }
    if branches.is_empty() {
        return Some(empty);
    }
    let home = NodeHome {
        rule: state.place,
        step: reader.rule.plan.emit.step,
    };
    let shape = bound_shape(&reader.rule, binding)?;
    let flattened = Flattened {
        branches,
        emptiness: Emptiness::Any(each.into()),
        keyless: Vec::new(),
    };
    let definition = defined(drafts, bound.node, home, shape, flattened);
    Some(use_of(drafts, definition, width))
}

/// Whether a rule reads a bound observation: it is flattened after every producer.
#[must_use]
pub(crate) fn reads_observations(rule: &AdmittedRule<'_>) -> bool {
    !rule.bound.is_empty()
}
