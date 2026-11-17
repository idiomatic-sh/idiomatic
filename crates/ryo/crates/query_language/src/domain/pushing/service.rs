//! The `pushing` service: free functions named by verb over the lookups into a join of edges and elements made before the join.

use super::types::ColumnPrefixLength;
use super::types::FilterDepth;
use super::types::FilterStep;
use super::types::FilteredElements;
use super::types::KeptColumns;
use super::types::Pushed;
use crate::domain::fusing::service::rides_the_walk;
use crate::domain::fusing::types::RightPredicatePlace;
use crate::domain::rewriting::types::Planned;
use crate::domain::rewriting::types::StepIndex;
use crate::domain::selection_pushing::service::{insert_before, renamed};
use ryo_relations::domain::relations::collections::relations::syntax_child::item::SyntaxChildColumn;
use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::types::{Column, Relation};
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::service::{column_place, fixed_prefix};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::RowWidth;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The steps with each lookup into a join of the edges of the tree with a filter of its elements made before the join:
/// asking the edges and the elements for the children of a few nodes does not need the children of every node.
/// `Join(left, Join(edges, elements))` keyed on the parent of the edges becomes `Join(Join(left, edges), elements)`. The
/// comparisons of the elements on the way ride the walk of the second join while none can fail (`rides_the_walk`); the rest
/// filter its answer after it, and when the answer does not hold a column they test the lookup stays as it is. That filter
/// runs a comparison that may fail only on the elements the left reaches, never on more than the join it replaces ran it on:
/// the push can remove a fault, never add one.
#[must_use]
pub(crate) fn push_lookups(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = push_lookups_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// The same rewrite with every externally observable root protected and renumbered together.
pub(crate) fn push_lookups_roots(mut steps: Vec<PlanStep>, roots: &mut [StepId]) -> Vec<PlanStep> {
    let mut place = 0;
    while place < steps.len() {
        if let Some(Pushed {
            below,
            above,
            residual,
        }) = pushed(&steps, StepIndex(place))
            && let Some(scope) = steps.get(place).map(|step| step.scope)
        {
            // The first of the two joins takes the place of the one it was made from, the second and any filter of its
            // answer follow it, and the last of them is what the readers of the one it was made from read.
            let step = |operation| PlanStep { operation, scope };
            let (before, last) = match residual {
                Some(residual) => (vec![step(below), step(above)], step(residual)),
                None => (vec![step(below)], step(above)),
            };
            let added = before.len();
            insert_before(&mut steps, roots, StepIndex(place), before, last);
            place += added;
        }
        place += 1;
    }
    steps
}

/// The comparisons `rest`, of the elements' columns, as comparisons of the columns of `answer`, the second join's answer
/// through `view`: none when the answer does not hold a column one of them tests.
fn over_the_answer(
    rest: &[PlanComparison],
    answer: &[JoinColumn],
    view: &[ColumnIndex],
) -> Option<Vec<PlanComparison>> {
    let answered = |column: ColumnIndex| -> Option<ColumnIndex> {
        let place = answer
            .iter()
            .position(|each| each.side == Side::Right && view_source(view, each.column) == Some(column))?;
        match u32::try_from(place) {
            Ok(at) => Some(ColumnIndex(at)),
            // A place past every column number names no column.
            Err(_past_every_column) => None,
        }
    };
    rest.iter()
        .map(|comparison| renamed(comparison, &answered))
        .collect()
}

/// A column of what the select at `from` of the chain reads, as a column of the join of edges and elements.
fn inward(chain: &[FilterStep<'_>], column: ColumnIndex, from: FilterDepth) -> Option<ColumnIndex> {
    let mut column = column;
    for FilterStep { kept, .. } in chain.iter().skip(from.0) {
        column = view_source(kept, column)?;
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
        let element_side = |column: ColumnIndex| -> Option<ColumnIndex> {
            let wanted = made_output
                .get(
                    usize::try_from(inward(chain, column, FilterDepth(depth + 1))?.0)
                        .unwrap_or(usize::MAX),
                )
                .copied()?;
            // A column past the elements' view is one the view does not make: the filter, and the push, decline.
            if wanted.side == Side::Right {
                view_source(element_view, wanted.column)
            } else {
                None
            }
        };
        for comparison in *comparisons {
            filters.push(renamed(comparison, &element_side)?);
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
    let mut kept_edge: Vec<ColumnIndex> =
        vec![column_place(Column::SyntaxChild(SyntaxChildColumn::Child))];
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

/// Where `column` is among `columns`, after `offset` columns before them: none when it is not among them, so a rewrite that asks
/// for a column it did not keep is refused instead of reading another column.
pub(crate) fn index_among(
    columns: &[ColumnIndex],
    column: ColumnIndex,
    offset: ColumnPrefixLength,
) -> Option<ColumnIndex> {
    let place = offset
        .0
        .checked_add(columns.iter().position(|each| *each == column)?)?;
    match u32::try_from(place) {
        Ok(at) => Some(ColumnIndex(at)),
        // A place past every column number names no column.
        Err(_past_every_column) => None,
    }
}

/// The columns the lookup gives: the left's where the answer reads them, the edges' or the elements' where the second join finds them.
fn answer_columns(
    output: &[JoinColumn],
    made_column: &dyn Fn(ColumnIndex) -> Option<JoinColumn>,
    at_left: &dyn Fn(ColumnIndex) -> Option<ColumnIndex>,
    at_edge: &dyn Fn(ColumnIndex) -> Option<ColumnIndex>,
) -> Option<Vec<JoinColumn>> {
    let mut answer = Vec::new();
    for column in output {
        answer.push(match column.side {
            Side::Left => JoinColumn {
                side: Side::Left,
                column: at_left(column.column)?,
            },
            Side::Right => {
                let wanted = made_column(column.column)?;
                match wanted.side {
                    Side::Left => JoinColumn {
                        side: Side::Left,
                        column: at_edge(wanted.column)?,
                    },
                    Side::Right => wanted,
                }
            }
        });
    }
    Some(answer)
}

/// The two joins that the join at `place` is made of, when it is a lookup into a join of edges and elements by the
/// parent of the edges, perhaps filtered on the way by comparisons of the elements; each reads the steps it follows as
/// they will be numbered once the first takes `place`.
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
    // The first of the two joins takes `place`; the second and the filter of its answer follow it.
    let first = StepId(u32::try_from(place.0).unwrap_or(u32::MAX));
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
    if *child != column_place(Column::SyntaxChild(SyntaxChildColumn::Child))
        || view_source(element_view, *element_key)
            != Some(column_place(Column::SyntaxTree(SyntaxTreeColumn::Id)))
    {
        return None;
    }
    let made_column = |column: ColumnIndex| -> Option<JoinColumn> {
        made_output
            .get(usize::try_from(inward(&chain, column, FilterDepth(0))?.0).unwrap_or(usize::MAX))
            .copied()
    };
    let folded = element_comparisons(
        &chain,
        made_output,
        FilteredElements {
            view: element_view,
            filter: element_filter,
        },
    )?;
    // The comparisons no element can fail ride the walk of the second join; the rest filter its answer.
    let walked =
        fixed_prefix(&folded, RowWidth(columns(Relation::SyntaxTree).len())).0.len();
    let (filters, rest) = folded.split_at(walked);
    if rides_the_walk(Relation::SyntaxTree, element_on, element_view, filters)
        == RightPredicatePlace::BelowAsSelect
    {
        return None;
    }
    // The key is the parent of the edge the made rows were found by.
    if made_column(*asked)?
        != (JoinColumn {
            side: Side::Left,
            column: column_place(Column::SyntaxChild(SyntaxChildColumn::Parent)),
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
    let residual = if rest.is_empty() {
        None
    } else {
        Some(Operation::Select {
            from: Input::Step(StepId(first.0 + 1)),
            comparisons: over_the_answer(rest, &answer, element_view)?,
            kept: Vec::new(),
        })
    };
    let above = Operation::Join {
        left: Input::Step(first),
        right: Input::Relation(Relation::SyntaxTree),
        left_on: vec![at_edge(column_place(Column::SyntaxChild(
            SyntaxChildColumn::Child,
        )))?],
        right_on: vec![*element_key],
        output: answer,
        filter: filters.to_vec(),
        view: element_view.clone(),

        reach: None,
    };
    Some(Pushed {
        below,
        above,
        residual,
    })
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
        right_on: vec![column_place(Column::SyntaxChild(SyntaxChildColumn::Parent))],
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
