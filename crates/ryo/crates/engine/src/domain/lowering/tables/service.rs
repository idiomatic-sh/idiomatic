//! The services of the `tables` part: whether a rule can be evaluated and written, step by step, and what each step's rows
//! look like. The `match` over `Operation` names every operation: the ones the lowering does not evaluate yet are listed
//! there, each by name, and refused as the step that cannot be evaluated. A rule is never refused for its cost or its size.

use crate::domain::lowering::nodes::service::{digested, input_shape, reached};
use crate::domain::lowering::nodes::types::{AdmittedRule, StepReach, StepShape};
use crate::domain::lowering::tables::types::TreeAxis;
use crate::domain::lowering::types::{LoadRefusal, PastWidth, PlanNode, SelectedRule};
use crate::domain::steps::types::{AxisDirection, TestOperation};
use ryo_relations::domain::catalog::service::{fact_form, node_column};
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::types::{CellKind, FactForm, NodeColumn, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Direction, Input, JoinColumn, Operand, PlanComparison, QueryLiteral, Side,
    StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// The comparison a plan's test is, as a step's or a stage's test; `Matches` takes its expression's number when written.
/// None for a test no step makes.
#[must_use]
pub(crate) const fn test_of(test: Test) -> Option<TestOperation> {
    Some(match test {
        Test::Equal => TestOperation::Equal,
        Test::NotEqual => TestOperation::NotEqual,
        Test::LessThan => TestOperation::Less,
        Test::LessThanOrEqual => TestOperation::LessOrEqual,
        Test::GreaterThan => TestOperation::Greater,
        Test::GreaterThanOrEqual => TestOperation::GreaterOrEqual,
        Test::In => TestOperation::In,
        Test::NotIn => TestOperation::NotIn,
        Test::GlobAny => TestOperation::Glob,
        Test::NotGlobAny => TestOperation::NotGlob,
        Test::StartsWith => TestOperation::StartsWith,
        Test::EndsWith => TestOperation::EndsWith,
        Test::Matches => TestOperation::Matches(crate::domain::steps::types::RegexId(0)),
        Test::NumericGreaterThanOrEqual => TestOperation::NumericAtLeast,
        Test::IntegerSuccessor => TestOperation::Successor,
        Test::NaturalLessThan => TestOperation::NaturalLess,
        Test::FloatTextGreaterThanOrEqual => return None,
    })
}

/// What an operand holds, over a row of `shape`: a column's kind, a literal's, a bound parameter's. None for an operand the
/// lowering writes no operand for: a function applied, a parameter bound to a list, a column past the row.
fn operand_kind(rule: &SelectedRule<'_>, shape: &StepShape, operand: &Operand) -> Option<CellKind> {
    match operand {
        Operand::Column(column) => shape.0.get(usize::from(*column)).copied(),
        Operand::Literal(QueryLiteral::Number(_)) => Some(CellKind::Number),
        Operand::Literal(QueryLiteral::Text(_)) => Some(CellKind::Symbol),
        Operand::Parameter(parameter) => {
            let place = usize::try_from(parameter.0).unwrap_or(usize::MAX);
            match rule.bindings.0.get(place)? {
                ParameterBinding::Number(_) => Some(CellKind::Number),
                ParameterBinding::Text(_) => Some(CellKind::Symbol),
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

/// The shape, when every comparison can be written over a row of it; none when one cannot.
fn comparable<'s>(
    rule: &SelectedRule<'_>,
    shape: &'s StepShape,
    comparisons: &[PlanComparison],
) -> Option<&'s StepShape> {
    let mut each = comparisons.iter();
    let written = each.all(|comparison| {
        let test = match (comparison.test, &comparison.right) {
            (Test::Matches, Operand::Literal(QueryLiteral::Text(_))) => true,
            (Test::Matches, _) => false,
            (test, _) => test_of(test).is_some(),
        };
        test && operand_kind(rule, shape, &comparison.left).is_some()
            && operand_kind(rule, shape, &comparison.right).is_some()
    });
    written.then_some(shape)
}

/// The kinds of a row at the given columns; none when a column is past the row.
fn picked(shape: &StepShape, columns: &[ColumnIndex]) -> Option<StepShape> {
    let kinds: Option<Vec<CellKind>> = columns
        .iter()
        .map(|column| shape.0.get(usize::from(*column)).copied())
        .collect();
    kinds.map(StepShape)
}

/// The kinds of a join's answer.
fn joined(left: &StepShape, right: &StepShape, output: &[JoinColumn]) -> Option<StepShape> {
    let kinds: Option<Vec<CellKind>> = output
        .iter()
        .map(|column| {
            let side = match column.side {
                Side::Left => left,
                Side::Right => right,
            };
            side.0.get(usize::from(column.column)).copied()
        })
        .collect();
    kinds.map(StepShape)
}

/// The left side's shape, when each key column is within its side's row and the two sides have as many keys.
fn keyed<'s>(
    left: &'s StepShape,
    right: &StepShape,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
) -> Option<&'s StepShape> {
    let held = left_on.len() == right_on.len()
        && picked(left, left_on).is_some()
        && picked(right, right_on).is_some();
    held.then_some(left)
}

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
        Direction::Child
        | Direction::Deep
        | Direction::PreviousSibling
        | Direction::NextSibling => None,
    }
}

/// What a step's rows look like, from its inputs' shapes; none when the lowering does not evaluate the step. Every operation
/// is named: the ones in the second half are what is left to lower.
fn shaped(
    rule: &SelectedRule<'_>,
    shapes: &[Option<StepShape>],
    operation: &Operation,
) -> Option<StepShape> {
    let of = |input: &Input| input_shape(shapes, *input);
    match operation {
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
            // A join against a closure (`reach`) and a join through a view of its right side are not lowered yet.
            let plain =
                view.is_empty() && reach.is_none() && comparable(rule, &right, filter).is_some();
            (plain && keyed(&left, &right, left_on, right_on).is_some())
                .then(|| joined(&left, &right, output))
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
            let (mut shape, domain) = (of(from)?, of(domain)?);
            // A walk over edges a step derives, and an axis with a view of its domain or its own answer columns, are not
            // lowered yet; nor are the axes `walk_of` names none for.
            let tree = tree_relation(edges, *parent, *child).is_some();
            let plain = domain_view.is_empty()
                && output.is_empty()
                && comparable(rule, &domain, domain_filter).is_some()
                && (domain_filter.is_empty() || walk_of(*direction) != Some(TreeAxis::Nearest));
            let reached = *domain.0.get(usize::from(*domain_id))?;
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
            // A closure over a relation's own rows (edges that are no step) is not lowered yet: its edges are a definition.
            let read = match edges {
                Input::Step(_) => of(edges)?,
                Input::Relation(_) | Input::BoundObservation(_) => return None,
            };
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
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Follow { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Worklist { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. } => None,
    }
}

/// A rule as the lowering reads it: every step its emit reaches has a shape. Refused, naming the first step in step order
/// that cannot be evaluated (an operation not lowered, a step that hands rows over to the project pass, a step out of
/// order), or the columns of a step when they are past the width a table's columns are counted in.
pub(crate) fn admitted<'a>(rule: SelectedRule<'a>) -> Result<AdmittedRule<'a>, LoadRefusal> {
    let plan = rule.plan;
    let unlowerable =
        |step| LoadRefusal::Unlowerable(PlanNode(u32::try_from(step).unwrap_or(u32::MAX)));
    let reached = reached(plan).map_err(|step| unlowerable(usize::from(step)))?;
    let mut shapes: Vec<Option<StepShape>> = vec![None; plan.steps.len()];
    for (place, step) in plan.steps.iter().enumerate() {
        if reached.get(place) != Some(&StepReach::Reached) {
            continue;
        }
        let per_file = step.scope == StepScope::PerFile && plan.frontier.is_empty();
        let shape = shaped(&rule, &shapes, &step.operation)
            .filter(|_| per_file)
            .ok_or_else(|| unlowerable(place))?;
        if shape.0.len() > usize::from(u16::MAX) {
            return Err(LoadRefusal::Budget(PastWidth::TableColumns));
        }
        if let Some(slot) = shapes.get_mut(place) {
            *slot = Some(shape);
        }
    }
    Ok(AdmittedRule {
        digests: digested(&rule),
        rule,
        shapes,
    })
}
