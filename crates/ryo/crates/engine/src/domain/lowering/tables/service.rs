//! The services of the `tables` part: whether a rule can be evaluated and written, step by step, and what each step's rows
//! look like. The `match` over `Operation` names every operation: the ones the lowering does not evaluate yet are listed
//! there, each by name, and refused as the step that cannot be evaluated. A rule is never refused for its cost or its size.

use crate::domain::lowering::nodes::service::{digested, reached};
use crate::domain::lowering::observing::service::{
    hosted_shape, input_kinds, local_of, rows_shape,
};
use crate::domain::lowering::observing::types::Local;
use crate::domain::lowering::nodes::types::{AdmittedRule, StepReach, StepShape};
use crate::domain::lowering::tables::shapes::service::{
    capture_slots, comparable, joined, keyed, operand_kind, picked, search_kinds, source_search,
    viewed,
};
use crate::domain::lowering::tables::types::TreeAxis;
use crate::domain::lowering::types::{LoadGiven, LoadRefusal, PastWidth, PlanNode, SelectedRule};
use crate::domain::steps::types::AxisDirection;
use ryo_relations::domain::catalog::service::{fact_form, node_column};
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::types::{CellKind, FactForm, NodeColumn, Relation};
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, Input};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_plans::service::read_order;

/// The node relation an axis walks the tree of: its edges are a relation of the node-table form whose parent and child
/// columns are the node table's parent and element. None for edges a step derives.
#[must_use]
pub(crate) fn tree_relation(
    edges: &Input,
    parent: ColumnIndex,
    child: ColumnIndex,
) -> Option<Relation> {
    let Input::Relation(relation) = edges else {
        return None;
    };
    let listed = columns(*relation);
    let column = |place: ColumnIndex| {
        listed
            .get(usize::from(place))
            .copied()
            .and_then(node_column)
    };
    let tree = fact_form(*relation) == FactForm::NodeTable
        && column(parent) == Some(NodeColumn::Parent)
        && column(child) == Some(NodeColumn::Id);
    tree.then_some(*relation)
}

/// How the lowering walks a plan's axis: along a tree axis, or to the nearest ancestor its domain holds. None for an axis
/// not lowered yet.
#[must_use]
pub(crate) const fn walk_of(direction: Direction) -> Option<TreeAxis> {
    match direction {
        Direction::Ancestor => Some(TreeAxis::Along(AxisDirection::Ancestor)),
        Direction::Descendant => Some(TreeAxis::Along(AxisDirection::Descendant)),
        Direction::NearestAncestor => Some(TreeAxis::Nearest),
        Direction::Parent => Some(TreeAxis::Along(AxisDirection::Parent)),
        Direction::NextSibling => Some(TreeAxis::Along(AxisDirection::NextSibling)),
        Direction::Child | Direction::Deep | Direction::PreviousSibling => None,
    }
}

/// What a step's rows look like, from its inputs' shapes; none when the lowering does not evaluate the step. Every operation
/// is named: the ones in the second half are what is left to lower.
fn shaped(
    rule: &SelectedRule<'_>,
    given: LoadGiven<'_>,
    shapes: &[Option<StepShape>],
    operation: &Operation,
) -> Option<StepShape> {
    let of = |input: &Input| input_kinds(rule, shapes, *input);
    match operation {
        // The host's rows and the rows a query wrote: constant tables (`observing`).
        Operation::Hosted { relation } => hosted_shape(given, *relation),
        Operation::Rows { .. } => rows_shape(operation),
        Operation::Select {
            from,
            comparisons,
            kept,
        } => {
            let shape = of(from)?;
            let _written = comparable(rule, &shape, comparisons)?;
            if kept.is_empty() {
                Some(shape)
            } else {
                picked(&shape, kept)
            }
        }
        Operation::Project { from, columns, .. } => picked(&of(from)?, columns),
        Operation::Extend { from, values } => {
            let mut shape = of(from)?;
            for value in values {
                let kind = operand_kind(rule, &shape, value)?;
                shape.0.push(kind);
            }
            Some(shape)
        }
        Operation::Join {
            left,
            right,
            left_on,
            right_on,
            output,
            filter,
            view,
            reach,
        } => {
            let (left, right) = (of(left)?, of(right)?);
            // A join against a closure (`reach`) is not lowered: the plan the engine receives holds the closure as a step.
            // The keys and the answer name the right side's columns through its view; the filter reads the whole row.
            let seen = viewed(&right, view)?;
            let plain = reach.is_none() && comparable(rule, &right, filter).is_some();
            (plain && keyed(&left, &seen, left_on, right_on).is_some())
                .then(|| joined(&left, &seen, output))
                .flatten()
        }
        Operation::Union { left, right } => {
            let (left, right) = (of(left)?, of(right)?);
            (left.0.len() == right.0.len()).then_some(left)
        }
        Operation::Antijoin {
            left,
            right,
            left_on,
            right_on,
        } => {
            let (left, right) = (of(left)?, of(right)?);
            keyed(&left, &right, left_on, right_on).cloned()
        }
        Operation::GroupCount {
            from,
            keys,
            distinct,
        } => {
            let read = of(from)?;
            // A count of a column's distinct values names a column of what it groups.
            let counted: Vec<ColumnIndex> = distinct.iter().copied().collect();
            let _within = picked(&read, &counted)?;
            let mut shape = picked(&read, keys)?;
            shape.0.push(CellKind::Number);
            Some(shape)
        }
        Operation::GroupBound {
            from, keys, order, ..
        } => {
            let read = of(from)?;
            let _within = (picked(&read, keys)?, picked(&read, &[*order])?);
            Some(read)
        }
        Operation::GroupOrdinal { from, keys, order } => {
            let mut read = of(from)?;
            let _within = (picked(&read, keys)?, picked(&read, &[*order])?);
            read.0.push(CellKind::Number);
            Some(read)
        }
        Operation::AsOf {
            left,
            right,
            left_on,
            right_on,
            left_order,
            right_order,
            output,
            ..
        } => {
            let (left, right) = (of(left)?, of(right)?);
            let ordered = picked(&left, &[*left_order]).is_some()
                && picked(&right, &[*right_order]).is_some();
            (ordered && keyed(&left, &right, left_on, right_on).is_some())
                .then(|| joined(&left, &right, output))
                .flatten()
        }
        Operation::Axis {
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
            output,
        } => {
            let domain_input = domain;
            let (mut shape, domain) = (of(from)?, of(domain)?);
            // An axis with its own answer columns is not lowered yet, nor are the axes `walk_of` names none for. The
            // entity column is named through the domain's view; the filter reads the whole row. Edges that are no tree of a node relation are climbed by a nearest axis and by an ancestor axis alone.
            let climbs = matches!(
                walk_of(*direction),
                Some(TreeAxis::Nearest | TreeAxis::Along(AxisDirection::Ancestor))
            );
            let tree = tree_relation(edges, *parent, *child).is_some()
                || (climbs && picked(&of(edges)?, &[*parent, *child]).is_some());
            // A nearest axis with a filter reads its domain in place, which a relation's rows can be and a step's are not.
            let plain = output.is_empty()
                && comparable(rule, &domain, domain_filter).is_some()
                && (domain_filter.is_empty()
                    || walk_of(*direction) != Some(TreeAxis::Nearest)
                    || matches!(domain_input, Input::Relation(_)));
            let reached = *domain
                .0
                .get(usize::from(view_source(domain_view, *domain_id)?))?;
            let _start = shape.0.get(usize::from(*id))?;
            (tree && plain && walk_of(*direction).is_some()).then(|| {
                shape.0.push(reached);
                shape
            })
        }
        Operation::Closure {
            edges,
            keys,
            source,
            destination,
        } => {
            let read = of(edges)?;
            let mut shape = picked(&read, keys)?;
            shape.0.extend(picked(&read, &[*source, *destination])?.0);
            Some(shape)
        }
        // The keys of the edges, then a source and a fixed point it reaches: the row of the `FixedPoints` graph table.
        Operation::Follow {
            edges,
            keys,
            source,
            destination,
        } => {
            let read = of(edges)?;
            let mut shape = picked(&read, keys)?;
            shape.0.extend(picked(&read, &[*source, *destination])?.0);
            Some(shape)
        }
        // The per-row meanings lowered: each appends what the meaning gives to the rows it reads.
        Operation::Replace { from, text, .. } => {
            let mut shape = of(from)?;
            let _read = picked(&shape, &[*text])?;
            shape.0.push(CellKind::Symbol);
            Some(shape)
        }
        Operation::SourceMatches {
            from,
            span,
            base_line,
            ..
        } => {
            let mut shape = of(from)?;
            let line = base_line.as_slice();
            let read = picked(&shape, &[*span])?.0 == [CellKind::Span]
                && picked(&shape, line)?
                    .0
                    .iter()
                    .all(|kind| *kind == CellKind::Number);
            let search = source_search(operation).filter(|_| read)?;
            shape.0.extend(search_kinds(search));
            Some(shape)
        }
        Operation::Capture {
            from,
            path,
            pattern,
        } => {
            let mut shape = of(from)?;
            let _read = picked(&shape, &[*path])?;
            let slots = capture_slots(pattern).0;
            shape.0.extend(std::iter::repeat_n(CellKind::Symbol, slots));
            Some(shape)
        }
        // An ordered match is a table of pairs, a left row then a right row; its answer names columns of either.
        Operation::OrderedMatch {
            left,
            right,
            left_on,
            right_on,
            left_order,
            right_order,
            output,
        } => {
            let (left, right) = (of(left)?, of(right)?);
            let ordered = picked(&left, &[*left_order])?.0 == [CellKind::Number]
                && picked(&right, &[*right_order])?.0 == [CellKind::Number];
            (ordered && keyed(&left, &right, left_on, right_on).is_some())
                .then(|| joined(&left, &right, output))
                .flatten()
        }
        Operation::SpanBetween { from, start, end } => {
            let mut shape = of(from)?;
            if picked(&shape, &[*start, *end])?.0 != [CellKind::Span; 2] {
                return None;
            }
            shape.0.push(CellKind::Span);
            Some(shape)
        }
        Operation::SpanBounds { from, span } => {
            let mut shape = of(from)?;
            let _read = picked(&shape, &[*span])?;
            shape.0.extend([CellKind::Number, CellKind::Number]);
            Some(shape)
        }
        // What is left to lower, each by name: no arm stands for the rest.
        Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::GroupDigest { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Worklist { .. }
        | Operation::Cycles { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. } => None,
    }
}

/// A rule as the lowering reads it: every step its emit reaches has a shape, and holds the rows of the file being scanned or
/// the same rows for every file (`observing::service::local_of`). Refused, naming the first step its emit reaches, in the
/// order the steps are read, that cannot be evaluated (an operation not lowered, a step that holds rows of several files at
/// once, a step out of order), or the columns of a step when they are past the width a table's columns are counted in; and
/// refused at its emit when the emitted rows are the same for every file. A plan may hand rows over to another rule's steps
/// (its `frontier`, an output another rule observes): the steps only that output reaches are shaped when a rule that binds
/// the output is accepted (`observing::service::resolved`), and the rule is judged by the steps its own emit reaches alone.
#[cfg(test)]
pub(crate) fn admitted<'a>(rule: SelectedRule<'a>) -> Result<AdmittedRule<'a>, LoadRefusal> {
    admitted_in(rule, LoadGiven::default())
}

/// `admitted`, with what the host declares beside the plans.
pub(crate) fn admitted_in<'a>(
    rule: SelectedRule<'a>,
    given: LoadGiven<'a>,
) -> Result<AdmittedRule<'a>, LoadRefusal> {
    let plan = rule.plan;
    let reached = reached(plan).map_err(|step| unlowerable(usize::from(step)))?;
    let mut admitted = AdmittedRule {
        digests: digested(&rule),
        rule,
        shapes: vec![None; plan.steps.len()],
        given,
        locals: vec![None; plan.steps.len()],
        observed: Vec::new(),
        bound: Vec::new(),
    };
    shape_reached(&mut admitted, &reached)?;
    let emitted = usize::from(plan.emit.step);
    if admitted.locals.get(emitted) != Some(&Some(Local::File)) {
        return Err(unlowerable(emitted));
    }
    Ok(admitted)
}

/// The refusal of a step, by its place.
fn unlowerable(step: usize) -> LoadRefusal {
    LoadRefusal::Unlowerable(PlanNode(u32::try_from(step).unwrap_or(u32::MAX)))
}

/// Shapes every step `reached` names that has no shape yet, each after the steps it reads (`read_order`), with whose rows
/// it holds. Refused as `admitted` refuses.
pub(crate) fn shape_reached(
    rule: &mut AdmittedRule<'_>,
    reached: &[StepReach],
) -> Result<(), LoadRefusal> {
    let plan = rule.rule.plan;
    // Every step after the steps it reads: a step that reads one with no shape yet is one that cannot be evaluated.
    for place in read_order(plan).into_iter().map(usize::from) {
        let Some(step) = plan.steps.get(place) else {
            continue;
        };
        if reached.get(place) != Some(&StepReach::Reached)
            || rule.shapes.get(place).is_some_and(Option::is_some)
        {
            continue;
        }
        let shape = shaped(&rule.rule, rule.given, &rule.shapes, &step.operation)
            .ok_or_else(|| unlowerable(place))?;
        let local = local_of(&rule.rule, rule.given, &rule.shapes, &rule.locals, step)
            .ok_or_else(|| unlowerable(place))?;
        if shape.0.len() > usize::from(u16::MAX) {
            return Err(LoadRefusal::Budget(PastWidth::TableColumns));
        }
        if let Some(slot) = rule.shapes.get_mut(place) {
            *slot = Some(shape);
        }
        if let Some(slot) = rule.locals.get_mut(place) {
            *slot = Some(local);
        }
    }
    Ok(())
}
