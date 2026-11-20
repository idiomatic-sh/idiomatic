//! The `plan_expansion` service: a plan with each operation this engine never runs (`EXPANDED`: `as-of`, `group-min` and
//! `group-max`, `group-ordinal`, `follow`) rewritten into the operations it does run, one canonical shape for each kind. A pure
//! function of the plan: no IO, no bindings, no rows. This engine is prepared from what it answers and from nothing else, so it
//! never meets one of those kinds; the plan as compiled is what every other reader of a plan reads. What each kind means is
//! stated once, in `ryo_operators::domain::ordered_groups` and `ryo_operators::domain::follow`: a shape written here answers
//! the rows those answer, wherever this engine can (a `follow` is followed 2^20 edges and no further).

use super::widths::service::width_of;
use ryo_operators::domain::physical_plan::constants::EXPANDED;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_orders::types::{GroupEnd, OrderBound};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, JoinColumn, Operand, PlanComparison, QueryLiteral,
    RepeatedRows, RowWidth, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::service::{each_input, kind_of};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_plans::types::Plan;

/// How many times the ladder of a `follow` joins its pointers to themselves: 2^20 hops, the depth the packs' ladder followed.
pub(crate) const LADDER_LINKS: usize = 20;

/// `plan` with every operation of an `EXPANDED` kind rewritten into the steps this engine runs, the steps after it renumbered,
/// and what the plan emits, hands over and exports pointing at the same rows. A plan that holds none is answered as it is. An
/// operation whose input's width the plan does not say (it reads the host's rows with no step between that names its columns)
/// is left as written: this engine refuses it when it would run, as a kind it does not lower.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::plan_expansion::service::expanded;
/// use ryo_values::domain::query_plans::types::Plan;
/// fn for_this_engine(plan: &Plan) -> Plan {
///     expanded(plan)
/// }
/// ```
#[must_use]
pub fn expanded(plan: &Plan) -> Plan {
    let expands = |step: &PlanStep| EXPANDED.contains(&kind_of(&step.operation));
    if !plan.steps.iter().any(expands) {
        return plan.clone();
    }
    let mut steps: Vec<PlanStep> = Vec::with_capacity(plan.steps.len());
    // The step of the expanded plan that answers each step of `plan`.
    let mut answers: Vec<StepId> = Vec::with_capacity(plan.steps.len());
    // A step reads the steps before it, which are answered already; a plan that names a later step is not one the compiler
    // makes, and the name is left as written for this engine to refuse.
    let renumbered = |answers: &[StepId], input: Input| match input {
        Input::Step(id) => match answers.get(usize::try_from(id.0).unwrap_or(usize::MAX)) {
            Some(answer) => Input::Step(*answer),
            None => input,
        },
        Input::Relation(_) | Input::BoundObservation(_) => input,
    };
    for step in &plan.steps {
        let mut operation = step.operation.clone();
        each_input(&mut operation, &mut |input| {
            *input = renumbered(&answers, *input);
        });
        let answer = written(plan, &mut steps, step.scope, &operation)
            .unwrap_or_else(|| pushed(&mut steps, step.scope, operation));
        answers.push(match answer {
            Input::Step(id) => id,
            Input::Relation(_) | Input::BoundObservation(_) => next(&steps),
        });
    }
    let mut out = plan.clone();
    out.steps = steps;
    if let Input::Step(emitted) = renumbered(&answers, Input::Step(plan.emit.step)) {
        out.emit.step = emitted;
    }
    for input in &mut out.frontier {
        *input = renumbered(&answers, *input);
    }
    for observation in &mut out.observations {
        observation.source = renumbered(&answers, observation.source);
    }
    out
}

/// The steps `operation` expands into, written after `steps` to run in `scope`, and the step that answers it; none for an
/// operation that is left as it is.
fn written(
    plan: &Plan,
    steps: &mut Vec<PlanStep>,
    scope: StepScope,
    operation: &Operation,
) -> Option<Input> {
    match operation {
        Operation::AsOf {
            left,
            right,
            left_on,
            right_on,
            left_order,
            right_order,
            bound,
            output,
        } => {
            let width = width_of(plan, steps, *left)?;
            // The band rows: every left column, the right order after them, then each other right column the answer names.
            let mut right_columns = vec![*right_order];
            for named in output.iter().filter(|named| named.side == Side::Right) {
                if !right_columns.contains(&named.column) {
                    right_columns.push(named.column);
                }
            }
            let named = output
                .iter()
                .map(|named| match named.side {
                    Side::Left => Some(named.column),
                    Side::Right => right_columns
                        .iter()
                        .position(|column| *column == named.column)
                        .map(|at| column_past(RowWidth(width.0 + at))),
                })
                .collect::<Option<Vec<_>>>()?;
            let mut band_output = left_columns(width);
            band_output.extend(right_columns.iter().map(|column| JoinColumn {
                side: Side::Right,
                column: *column,
            }));
            let band_width = RowWidth(band_output.len());
            let pairs = joined(*left, *right, left_on, right_on, band_output);
            let pairs = pushed(steps, scope, pairs);
            // The right order against the left order, as the bound says; then, of the pairs of one left row, the ones another
            // pair is nearer than: the nearest are what no pair is nearer than, every one of them when they tie.
            let (within, nearer) = match bound {
                OrderBound::Before => (Test::LessThan, Test::LessThan),
                OrderBound::AtOrBefore => (Test::LessThanOrEqual, Test::LessThan),
                OrderBound::After => (Test::GreaterThan, Test::GreaterThan),
                OrderBound::AtOrAfter => (Test::GreaterThanOrEqual, Test::GreaterThan),
            };
            let band = compared(pairs, column_past(width), within, *left_order);
            let band = pushed(steps, scope, band);
            let right_order = column_past(width);
            let kept = undominated(
                steps,
                scope,
                band,
                &every(width),
                right_order,
                nearer,
                band_width,
            );
            Some(pushed(
                steps,
                scope,
                projected(kept, named, RepeatedRows::Kept),
            ))
        }
        Operation::GroupBound {
            from,
            keys,
            order,
            end,
        } => {
            let width = width_of(plan, steps, *from)?;
            let passed = match end {
                GroupEnd::Greatest => Test::LessThan,
                GroupEnd::Least => Test::GreaterThan,
            };
            Some(undominated(
                steps, scope, *from, keys, *order, passed, width,
            ))
        }
        Operation::GroupOrdinal { from, keys, order } => {
            let width = width_of(plan, steps, *from)?;
            let every = every(width);
            let rows = projected(*from, every.clone(), RepeatedRows::Dropped);
            let rows = pushed(steps, scope, rows);
            // Each row beside each row of its group, with that row's order: the pairs whose other order is strictly smaller.
            let mut beside = left_columns(width);
            beside.push(JoinColumn {
                side: Side::Right,
                column: *order,
            });
            let pairs = pushed(steps, scope, joined(rows, rows, keys, keys, beside));
            let earlier = compared(pairs, column_past(width), Test::LessThan, *order);
            let earlier = pushed(steps, scope, earlier);
            let counted = Operation::GroupCount {
                from: earlier,
                keys: every.clone(),
                distinct: None,
            };
            let counted = pushed(steps, scope, counted);
            // The rows no row of their group is below are counted by no pair: their place is 0.
            let first = pushed(steps, scope, antijoined(rows, counted, &every));
            let zero = Operation::Extend {
                from: first,
                values: vec![Operand::Literal(QueryLiteral::Number(Integer(0)))],
            };
            let zero = pushed(steps, scope, zero);
            let placed = Operation::Union {
                left: counted,
                right: zero,
            };
            Some(pushed(steps, scope, placed))
        }
        Operation::Follow {
            edges,
            keys,
            source,
            destination,
        } => {
            // The distinct pointers, their keys first; then the ladder: each link joins the pointers to themselves where one
            // ends and another starts, so the pointers of a link follow twice as far as those of the link before.
            let (keyed, to_source) = (RowWidth(keys.len()), RowWidth(keys.len() + 1));
            let mut kept = keys.clone();
            kept.extend([*source, *destination]);
            let mut pointers = pushed(steps, scope, projected(*edges, kept, RepeatedRows::Dropped));
            let mut from = every(keyed);
            from.push(column_past(to_source));
            let onto = every(to_source);
            let mut output = left_columns(to_source);
            output.push(JoinColumn {
                side: Side::Right,
                column: column_past(to_source),
            });
            for _ in 0..LADDER_LINKS {
                let link = joined(pointers, pointers, &from, &onto, output.clone());
                pointers = pushed(steps, scope, link);
            }
            let every = every(RowWidth(keys.len() + 2));
            Some(pushed(
                steps,
                scope,
                projected(pointers, every, RepeatedRows::Dropped),
            ))
        }
        Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
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

/// The rows of `source`, `width` columns wide, that no row of their group by `keys` passes in `order`: a row is passed when
/// its order stands in `passed` to another's. Written as the packs spell an extremum (a self join on the keys, the one strict
/// comparison, the distinct passed rows, an antijoin on every column), the shape this engine's ordered probe recognises, so a
/// repeated row is kept as an antijoin keeps it.
fn undominated(
    steps: &mut Vec<PlanStep>,
    scope: StepScope,
    source: Input,
    keys: &[ColumnIndex],
    order: ColumnIndex,
    passed: Test,
    width: RowWidth,
) -> Input {
    let every = every(width);
    let mut beside = left_columns(width);
    beside.push(JoinColumn {
        side: Side::Right,
        column: order,
    });
    let pairs = pushed(steps, scope, joined(source, source, keys, keys, beside));
    let behind = compared(pairs, order, passed, column_past(width));
    let behind = pushed(steps, scope, behind);
    let dominated = projected(behind, every.clone(), RepeatedRows::Dropped);
    let dominated = pushed(steps, scope, dominated);
    pushed(steps, scope, antijoined(source, dominated, &every))
}

/// Writes `operation` after `steps`, to run in `scope`, and answers the step it is.
fn pushed(steps: &mut Vec<PlanStep>, scope: StepScope, operation: Operation) -> Input {
    let id = next(steps);
    steps.push(PlanStep { operation, scope });
    Input::Step(id)
}

/// The step the next one written after `steps` will be.
fn next(steps: &[PlanStep]) -> StepId {
    StepId(u32::try_from(steps.len()).unwrap_or(u32::MAX))
}

/// The column that follows the columns of a row `width` wide.
fn column_past(width: RowWidth) -> ColumnIndex {
    ColumnIndex(u32::try_from(width.0).unwrap_or(u32::MAX))
}

/// Every column of a row `width` wide, in order.
fn every(width: RowWidth) -> Vec<ColumnIndex> {
    (0..width.0)
        .map(|place| column_past(RowWidth(place)))
        .collect()
}

/// Every column of the left side of a join whose left rows are `width` wide, in order.
fn left_columns(width: RowWidth) -> Vec<JoinColumn> {
    every(width)
        .into_iter()
        .map(|column| JoinColumn {
            side: Side::Left,
            column,
        })
        .collect()
}

/// A plain join of `left` and `right` on equal keys, answering `output`.
fn joined(
    left: Input,
    right: Input,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    output: Vec<JoinColumn>,
) -> Operation {
    Operation::Join {
        left,
        right,
        left_on: left_on.to_vec(),
        right_on: right_on.to_vec(),
        output,
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    }
}

/// The rows of `from` whose column `left` stands in `test` to their column `right`, every column kept. The comparison is no
/// text of the query, so it records no origin.
fn compared(from: Input, left: ColumnIndex, test: Test, right: ColumnIndex) -> Operation {
    Operation::Select {
        from,
        comparisons: vec![PlanComparison {
            test,
            left: Operand::Column(left),
            right: Operand::Column(right),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: Vec::new(),
    }
}

/// The `columns` of the rows of `from`.
fn projected(from: Input, columns: Vec<ColumnIndex>, distinct: RepeatedRows) -> Operation {
    Operation::Project {
        from,
        columns,
        distinct,
    }
}

/// The rows of `left` that no row of `right` equals on `every` column.
fn antijoined(left: Input, right: Input, every: &[ColumnIndex]) -> Operation {
    Operation::Antijoin {
        left,
        right,
        left_on: every.to_vec(),
        right_on: every.to_vec(),
    }
}
