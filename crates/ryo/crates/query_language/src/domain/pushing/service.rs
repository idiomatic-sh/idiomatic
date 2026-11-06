// the lookups pushed into a join and the filters pushed before a capture are the two pushes the planner makes and share the reading of a chain of filters.
//! The `pushing` service: free functions named by verb over the lookups into a join of edges and elements made before the join.

use ryo_relations::domain::relations::types::Relation;

use super::types::{
    ColumnPrefixLength, FilterDepth, FilterStep, FilteredElements, KeptColumns, MovedComparisons,
    Pushed,
};
use crate::domain::fusing::service::source_column;
use crate::domain::rewriting::types::{Planned, StepIndex};
use crate::domain::walking::service::width_of;
use ryo_values::domain::plan_parts::service::{columns_of, each_column, remapped};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::service::each_input;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The steps with each lookup into a join of the edges of the tree with a filter of its elements made before the join:
/// asking the edges and the elements for the children of a few nodes does not need the children of every node.
/// `Join(left, Join(edges, elements))` keyed on the parent of the edges becomes `Join(Join(left, edges), elements)`.
#[must_use]
pub(crate) fn push_lookups(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = push_lookups_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// The same rewrite with every externally observable root protected and renumbered together.
pub(crate) fn push_lookups_roots(mut steps: Vec<PlanStep>, roots: &mut [StepId]) -> Vec<PlanStep> {
    let mut place = 0;
    while place < steps.len() {
        if let Some(Pushed { below, above }) = pushed(&steps, StepIndex(place)) {
            // The first of the two joins takes the place of the one it was made from, and the rest follow it.
            let first = StepId(u32::try_from(place).unwrap_or(u32::MAX));
            for step in &mut steps {
                each_input(&mut step.operation, &mut |input| {
                    if let Input::Step(id) = input
                        && id.0 >= first.0
                    {
                        id.0 += 1;
                    }
                });
            }
            for root in roots.iter_mut() {
                if root.0 >= first.0 {
                    root.0 += 1;
                }
            }
            #[expect(
                clippy::indexing_slicing,
                reason = "`place` is below the steps' length, checked by the loop; the inputs were already renumbered, so stopping here would leave a wrong plan"
            )]
            let step = &mut steps[place];
            step.operation = above_with(above, first);
            let scope = step.scope;
            steps.insert(
                place,
                PlanStep {
                    operation: below,
                    scope,
                },
            );
            place += 1;
        }
        place += 1;
    }
    steps
}

/// The second join, reading the first at `first`.
fn above_with(mut operation: Operation, first: StepId) -> Operation {
    if let Operation::Join { left, .. } = &mut operation {
        *left = Input::Step(first);
    }
    operation
}

/// A column of what the select at `from` of the chain reads, as a column of the join of edges and elements.
fn inward(chain: &[FilterStep<'_>], column: ColumnIndex, from: FilterDepth) -> Option<ColumnIndex> {
    let mut column = column;
    for FilterStep { kept, .. } in chain.iter().skip(from.0) {
        if !kept.is_empty() {
            column = *kept.get(usize::try_from(column.0).unwrap_or(usize::MAX))?;
        }
    }
    Some(column)
}

/// The comparisons of the filters in between, as comparisons of the elements, after those the elements are already filtered by.
fn element_comparisons(
    chain: &[FilterStep<'_>],
    made_output: &[JoinColumn],
    elements: FilteredElements<'_>,
) -> Option<Vec<PlanComparison>> {
    let FilteredElements {
        view: element_view,
        filter: element_filter,
    } = elements;
    let mut filters: Vec<PlanComparison> = element_filter.to_vec();
    for (depth, FilterStep { comparisons, .. }) in chain.iter().enumerate() {
        for comparison in *comparisons {
            let element_side = |operand: &Operand| -> Option<Operand> {
                remapped(operand, &|column| {
                    let wanted = made_output
                        .get(
                            usize::try_from(inward(chain, column, FilterDepth(depth + 1))?.0)
                                .unwrap_or(usize::MAX),
                        )
                        .copied()?;
                    (wanted.side == Side::Right).then(|| {
                        source_column(element_view, wanted.column).unwrap_or(wanted.column)
                    })
                })
            };
            filters.push(PlanComparison {
                test: comparison.test,
                left: element_side(&comparison.left)?,
                right: element_side(&comparison.right)?,
                origin: comparison.origin,
            });
        }
    }
    Some(filters)
}

/// What the first join of a lookup keeps: the columns of the left the answer reads, then the columns of the edges the answer and the
/// second join read.
fn kept_columns(
    output: &[JoinColumn],
    made_column: &dyn Fn(ColumnIndex) -> Option<JoinColumn>,
) -> Option<KeptColumns> {
    let mut kept_left: Vec<ColumnIndex> = Vec::new();
    let mut kept_edge: Vec<ColumnIndex> = vec![ColumnIndex(1)];
    for column in output {
        match column.side {
            Side::Left => {
                if !kept_left.contains(&column.column) {
                    kept_left.push(column.column);
                }
            }
            Side::Right => {
                let wanted = made_column(column.column)?;
                if wanted.side == Side::Left && !kept_edge.contains(&wanted.column) {
                    kept_edge.push(wanted.column);
                }
            }
        }
    }
    Some(KeptColumns {
        left: kept_left,
        edge: kept_edge,
    })
}

/// Where `column` is among `columns`, after `offset` columns before them.
fn index_among(
    columns: &[ColumnIndex],
    column: ColumnIndex,
    offset: ColumnPrefixLength,
) -> ColumnIndex {
    ColumnIndex(
        u32::try_from(offset.0 + columns.iter().position(|each| *each == column).unwrap_or(0))
            .unwrap_or(u32::MAX),
    )
}

/// The columns the lookup gives: the left's where the answer reads them, the edges' or the elements' where the second join finds them.
fn answer_columns(
    output: &[JoinColumn],
    made_column: &dyn Fn(ColumnIndex) -> Option<JoinColumn>,
    at_left: &dyn Fn(ColumnIndex) -> ColumnIndex,
    at_edge: &dyn Fn(ColumnIndex) -> ColumnIndex,
) -> Option<Vec<JoinColumn>> {
    let mut answer = Vec::new();
    for column in output {
        answer.push(match column.side {
            Side::Left => JoinColumn {
                side: Side::Left,
                column: at_left(column.column),
            },
            Side::Right => {
                let wanted = made_column(column.column)?;
                match wanted.side {
                    Side::Left => JoinColumn {
                        side: Side::Left,
                        column: at_edge(wanted.column),
                    },
                    Side::Right => wanted,
                }
            }
        });
    }
    Some(answer)
}

/// The two joins that the join at `place` is made of, when it is a lookup into a join of edges and elements by the
/// parent of the edges, perhaps filtered on the way by comparisons of the elements.
fn pushed(steps: &[PlanStep], place: StepIndex) -> Option<Pushed> {
    let Operation::Join {
        left,
        right: Input::Step(read),
        left_on,
        right_on,
        output,
        filter,
        view,

        reach: None,
    } = &steps.get(place.0)?.operation
    else {
        return None;
    };
    if !filter.is_empty() || !view.is_empty() {
        return None;
    }
    let ([lookup], [asked]) = (left_on.as_slice(), right_on.as_slice()) else {
        return None;
    };
    // The filters between the join and the join of edges and elements: outermost first.
    let mut chain: Vec<FilterStep<'_>> = Vec::new();
    let mut at = *read;
    let made = loop {
        let step = steps.get(usize::try_from(at.0).unwrap_or(usize::MAX))?;
        if step.scope != steps.get(place.0)?.scope {
            return None;
        }
        let Operation::Select {
            from: Input::Step(inner),
            comparisons,
            kept,
        } = &step.operation
        else {
            break &step.operation;
        };
        chain.push(FilterStep { comparisons, kept });
        at = *inner;
    };
    let Operation::Join {
        left: Input::Relation(Relation::SyntaxChild),
        right: Input::Relation(Relation::SyntaxTree),
        left_on: edge_on,
        right_on: element_on,
        output: made_output,
        filter: element_filter,
        view: element_view,

        reach: None,
    } = made
    else {
        return None;
    };
    let ([child], [element_key]) = (edge_on.as_slice(), element_on.as_slice()) else {
        return None;
    };
    if *child != ColumnIndex(1) || source_column(element_view, *element_key) != Some(ColumnIndex(0))
    {
        return None;
    }
    let made_column = |column: ColumnIndex| -> Option<JoinColumn> {
        made_output
            .get(usize::try_from(inward(&chain, column, FilterDepth(0))?.0).unwrap_or(usize::MAX))
            .copied()
    };
    let filters = element_comparisons(
        &chain,
        made_output,
        FilteredElements {
            view: element_view,
            filter: element_filter,
        },
    )?;
    // The key is the parent of the edge the made rows were found by.
    if made_column(*asked)?
        != (JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        })
    {
        return None;
    }
    // What the first join keeps: the columns of the left the answer reads, then the columns of the edges the answer
    // and the second join read.
    let KeptColumns {
        left: kept_left,
        edge: kept_edge,
    } = kept_columns(output, &made_column)?;
    let width = ColumnPrefixLength(kept_left.len());
    let at_left = |column: ColumnIndex| index_among(&kept_left, column, ColumnPrefixLength(0));
    let at_edge = |column: ColumnIndex| index_among(&kept_edge, column, width);
    let below = edge_lookup(*left, *lookup, &kept_left, &kept_edge);
    let answer = answer_columns(output, &made_column, &at_left, &at_edge)?;
    let above = Operation::Join {
        left: Input::Step(StepId(0)),
        right: Input::Relation(Relation::SyntaxTree),
        left_on: vec![at_edge(ColumnIndex(1))],
        right_on: vec![*element_key],
        output: answer,
        filter: filters,
        view: element_view.clone(),

        reach: None,
    };
    Some(Pushed { below, above })
}

/// The first of the two joins: the rows of `left` met with their edges by `lookup`, keeping the columns of the left and of the edges
/// that the answer and the second join read.
fn edge_lookup(
    left: Input,
    lookup: ColumnIndex,
    kept_left: &[ColumnIndex],
    kept_edge: &[ColumnIndex],
) -> Operation {
    Operation::Join {
        left,
        right: Input::Relation(Relation::SyntaxChild),
        left_on: vec![lookup],
        right_on: vec![ColumnIndex(0)],
        output: kept_left
            .iter()
            .map(|column| JoinColumn {
                side: Side::Left,
                column: *column,
            })
            .chain(kept_edge.iter().map(|column| JoinColumn {
                side: Side::Right,
                column: *column,
            }))
            .collect(),
        filter: Vec::new(),
        view: Vec::new(),

        reach: None,
    }
}

/// The plan with each filter on the columns a capture was given, made before the capture instead of after it: the capture matches the
/// paths of the rows the filter lets through, not of every row. Not done when another step reads the capture.
#[must_use]
pub(crate) fn push_below_captures(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = push_below_captures_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// The same rewrite with every externally observable root protected and renumbered together.
pub(crate) fn push_below_captures_roots(
    mut steps: Vec<PlanStep>,
    roots: &mut [StepId],
) -> Vec<PlanStep> {
    let mut place = 0;
    while place < steps.len() {
        if let Some(MovedComparisons {
            capture,
            input,
            before: moved,
            rest,
        }) = movable(&steps, StepIndex(place), roots)
        {
            let capture = capture.0;
            let before = StepId(u32::try_from(capture).unwrap_or(u32::MAX));
            for step in &mut steps {
                each_input(&mut step.operation, &mut |read| {
                    if let Input::Step(id) = read
                        && id.0 >= before.0
                    {
                        id.0 += 1;
                    }
                });
            }
            for root in roots.iter_mut() {
                if root.0 >= before.0 {
                    root.0 += 1;
                }
            }
            #[expect(
                clippy::indexing_slicing,
                clippy::wildcard_enum_match_arm,
                clippy::unreachable,
                reason = "`movable` found a select at `place` reading a capture at `capture`; the inputs were already renumbered, so any other step here is a bug that must crash, not a wrong plan"
            )]
            {
                match &mut steps[place].operation {
                    Operation::Select { comparisons, .. } => *comparisons = rest,
                    _ => unreachable!(),
                }
                match &mut steps[capture].operation {
                    Operation::Capture { from, .. } => *from = Input::Step(before),
                    _ => unreachable!(),
                }
            }
            #[expect(
                clippy::indexing_slicing,
                reason = "`movable` found a capture at `capture`"
            )]
            let captured = &steps[capture];
            let scope = captured.scope;
            steps.insert(
                capture,
                PlanStep {
                    operation: Operation::Select {
                        from: input,
                        comparisons: moved,
                        kept: Vec::new(),
                    },
                    scope,
                },
            );
            place += 1;
        }
        place += 1;
    }
    steps
}

/// The filter at `place`, when it filters what a capture made on columns the capture was given: the capture, what it reads, and the
/// comparisons that move before it and those that stay.
fn movable(steps: &[PlanStep], place: StepIndex, roots: &[StepId]) -> Option<MovedComparisons> {
    let Operation::Select {
        from: Input::Step(read),
        comparisons,
        ..
    } = &steps.get(place.0)?.operation
    else {
        return None;
    };
    let capture = usize::try_from(read.0).unwrap_or(usize::MAX);
    let captured = steps.get(capture)?;
    let Operation::Capture { from, .. } = &captured.operation else {
        return None;
    };
    // Another step that reads the capture, or an emit of it, would see the rows the filter would take out.
    let readers = steps
        .iter()
        .filter(|step| {
            let mut reads = false;
            let mut copy = step.operation.clone();
            each_input(&mut copy, &mut |each| reads |= *each == Input::Step(*read));
            reads
        })
        .count();
    if readers != 1 || roots.contains(read) || captured.scope != steps.get(place.0)?.scope {
        return None;
    }
    let width = width_of(steps, *from)?.0;
    let (moved, rest): (Vec<PlanComparison>, Vec<PlanComparison>) =
        comparisons.iter().cloned().partition(|comparison| {
            let mut inside = true;
            for operand in [&comparison.left, &comparison.right] {
                each_column(operand, &mut |column| inside &= column.0 < width);
            }
            inside
        });
    // A filter that reads no column is no reason to move anything.
    let reads_a_column = moved.iter().any(|comparison| {
        !columns_of(&comparison.left).is_empty() || !columns_of(&comparison.right).is_empty()
    });
    (reads_a_column).then_some(MovedComparisons {
        capture: StepIndex(capture),
        input: *from,
        before: moved,
        rest,
    })
}
