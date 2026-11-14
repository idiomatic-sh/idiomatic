//! The `fusing` service: free functions named by verb over the filters and lookups of a plan that are read where they are found, instead of being made into tables first.

use ryo_relations::domain::relations::collections::relations::syntax_child::item::SyntaxChildColumn;
use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use ryo_relations::domain::relations::types::{Column, Relation};

use super::types::TreeLookup;
use crate::domain::rewriting::types::StepIndex;
use ryo_values::domain::plan_parts::service::{column_place, relation_column, remapped};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// Physical demand propagation runs after ordinary column/pruning rewrites, then use counts are recomputed.
pub(crate) fn fuse_keyed_reaches(
    steps: Vec<PlanStep>,
    protected: &[ryo_values::domain::plan_parts::types::StepId],
) -> Vec<PlanStep> {
    super::types::demand::value::DemandClosure::new(&steps, protected).rewrite()
}

/// The column of a step's input that the column `column` of its output is: the same one when the
/// step keeps every column, else the one it keeps there.
#[must_use]
pub(crate) fn source_column(kept: &[ColumnIndex], column: ColumnIndex) -> Option<ColumnIndex> {
    if kept.is_empty() {
        Some(column)
    } else {
        kept.get(usize::try_from(column.0).unwrap_or(usize::MAX))
            .copied()
    }
}

/// `consumer` as one step with `producer`, when both only filter and cut columns: the comparisons of
/// the producer, then the consumer's on the columns they name in the producer's input.
fn fuse(producer: &Operation, consumer: &Operation) -> Option<Operation> {
    let Operation::Select {
        from,
        comparisons: first,
        kept: cut,
    } = producer
    else {
        return None;
    };
    let Operation::Select {
        comparisons: second,
        kept: last,
        ..
    } = consumer
    else {
        return None;
    };
    let remap = |operand: &Operand| remapped(operand, &|column| source_column(cut, column));
    let mut comparisons = first.clone();
    for comparison in second {
        comparisons.push(PlanComparison {
            test: comparison.test,
            left: remap(&comparison.left)?,
            right: remap(&comparison.right)?,
            origin: comparison.origin,
        });
    }
    let kept = if last.is_empty() {
        cut.clone()
    } else {
        last.iter()
            .map(|column| source_column(cut, *column))
            .collect::<Option<Vec<_>>>()?
    };
    Some(Operation::Select {
        from: *from,
        comparisons,
        kept,
    })
}

/// The join at `place` with the right input read where its keys say, when that input is a filter of the
/// elements or of the edges of the syntax tree and the join asks for them by what finds them in the tree
/// (an element's id, an edge's parent or child): they are not made into a table.
fn fuse_join(steps: &[PlanStep], place: StepIndex) -> Option<Operation> {
    let Operation::Join {
        left,
        right: Input::Step(read),
        left_on,
        right_on,
        output,
        filter: no_filter,
        view: no_view,

        reach: None,
    } = &steps.get(place.0)?.operation
    else {
        return None;
    };
    let producer = steps.get(usize::try_from(read.0).unwrap_or(usize::MAX))?;
    let Operation::Select {
        from: Input::Relation(relation @ (Relation::SyntaxTree | Relation::SyntaxChild)),
        comparisons,
        kept,
    } = &producer.operation
    else {
        return None;
    };
    if producer.scope != steps.get(place.0)?.scope
        || !no_filter.is_empty()
        || !no_view.is_empty()
        || tree_lookup_supported(*relation, right_on, kept) == TreeLookup::Unsupported
    {
        return None;
    }
    Some(Operation::Join {
        left: *left,
        right: Input::Relation(*relation),
        left_on: left_on.clone(),
        right_on: right_on.clone(),
        output: output.clone(),
        filter: comparisons.clone(),
        view: kept.clone(),

        reach: None,
    })
}

/// Whether the resident tree lookup implements these ordered keys through this view.
/// Other joins must keep their filter as a separate select for the generic executor.
fn tree_lookup_supported(
    relation: Relation,
    keys: &[ColumnIndex],
    view: &[ColumnIndex],
) -> TreeLookup {
    // The column of `relation` that the key `key` reads through the view.
    let read = |key: ColumnIndex| relation_column(relation, source_column(view, key)?);
    let supported = match keys {
        [key] => matches!(
            read(*key),
            Some(
                Column::SyntaxTree(SyntaxTreeColumn::Id | SyntaxTreeColumn::Span)
                    | Column::SyntaxChild(SyntaxChildColumn::Parent | SyntaxChildColumn::Child)
            )
        ),
        [file, stretch] => matches!(
            (read(*file), read(*stretch)),
            (
                Some(Column::SyntaxTree(SyntaxTreeColumn::File)),
                Some(Column::SyntaxTree(SyntaxTreeColumn::Span))
            ) | (
                Some(Column::SyntaxTree(SyntaxTreeColumn::Span)),
                Some(Column::SyntaxTree(SyntaxTreeColumn::File))
            )
        ),
        _ => false,
    };
    if supported {
        TreeLookup::Supported
    } else {
        TreeLookup::Unsupported
    }
}

/// The axis at `place` answering among the rows of a filter of the elements of the syntax tree, with that
/// filter made the axis's own: the entities it reaches are decided where they are, and no table of the
/// elements is made.
fn fuse_axis(steps: &[PlanStep], place: StepIndex) -> Option<Operation> {
    let Operation::Axis {
        from,
        edges,
        domain: Input::Step(read),
        direction,
        id,
        parent,
        child,
        domain_id,
        domain_filter: no_filter,
        domain_view: no_view,
        output: no_output,
    } = &steps.get(place.0)?.operation
    else {
        return None;
    };
    let producer = steps.get(usize::try_from(read.0).unwrap_or(usize::MAX))?;
    let Operation::Select {
        from: Input::Relation(Relation::SyntaxTree),
        comparisons,
        kept,
    } = &producer.operation
    else {
        return None;
    };
    if producer.scope != steps.get(place.0)?.scope
        || !no_filter.is_empty()
        || !no_view.is_empty()
        || !no_output.is_empty()
        || source_column(kept, *domain_id)
            != Some(column_place(Column::SyntaxTree(SyntaxTreeColumn::Id)))
    {
        return None;
    }
    Some(Operation::Axis {
        from: *from,
        edges: *edges,
        domain: Input::Relation(Relation::SyntaxTree),
        direction: *direction,
        id: *id,
        parent: *parent,
        child: *child,
        domain_id: *domain_id,
        domain_filter: comparisons.clone(),
        domain_view: kept.clone(),
        output: Vec::new(),
    })
}

/// The filter at `place` over the rows of a join of the syntax tree, made the join's own: the join keeps only the rows that
/// the filter would have kept, when the filter asks only of columns of the tree.
fn fuse_join_filter(steps: &[PlanStep], place: StepIndex) -> Option<Operation> {
    let Operation::Select {
        from: Input::Step(read),
        comparisons,
        kept,
    } = &steps.get(place.0)?.operation
    else {
        return None;
    };
    let producer = steps.get(usize::try_from(read.0).unwrap_or(usize::MAX))?;
    let Operation::Join {
        left,
        right: right @ Input::Relation(Relation::SyntaxTree | Relation::SyntaxChild),
        left_on,
        right_on,
        output,
        filter,
        view,

        reach: None,
    } = &producer.operation
    else {
        return None;
    };
    let Input::Relation(relation) = right else {
        return None;
    };
    if producer.scope != steps.get(place.0)?.scope
        || tree_lookup_supported(*relation, right_on, view) == TreeLookup::Unsupported
    {
        return None;
    }
    // A column of the join's answer is a column of the tree when it comes from the right.
    let in_tree = |column: ColumnIndex| -> Option<ColumnIndex> {
        let answered = output.get(usize::try_from(column.0).unwrap_or(usize::MAX))?;
        (answered.side == Side::Right)
            .then(|| source_column(view, answered.column))
            .flatten()
    };
    let remap = |operand: &Operand| remapped(operand, &in_tree);
    let mut merged = filter.clone();
    for comparison in comparisons {
        merged.push(PlanComparison {
            test: comparison.test,
            left: remap(&comparison.left)?,
            right: remap(&comparison.right)?,
            origin: comparison.origin,
        });
    }
    let answer = if kept.is_empty() {
        output.clone()
    } else {
        kept.iter()
            .map(|column| {
                output
                    .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                    .copied()
            })
            .collect::<Option<Vec<_>>>()?
    };
    Some(Operation::Join {
        left: *left,
        right: *right,
        left_on: left_on.clone(),
        right_on: right_on.clone(),
        output: answer,
        filter: merged,
        view: view.clone(),

        reach: None,
    })
}

/// The steps with each filter of the rows another filter kept made one filter of what the first reads,
/// so the rows between them are never built. The step it was fused from stays for any other reader;
/// the pruning of unused steps drops it when there is none.
#[must_use]
pub(crate) fn fuse_selects(mut steps: Vec<PlanStep>) -> Vec<PlanStep> {
    for place in 0..steps.len() {
        if let Some(fused) = fuse_join(&steps, StepIndex(place))
            .or_else(|| fuse_axis(&steps, StepIndex(place)))
            .or_else(|| fuse_join_filter(&steps, StepIndex(place)))
        {
            if let Some(step) = steps.get_mut(place) {
                step.operation = fused;
            }
            continue;
        }
        let Some(step) = steps.get(place) else {
            continue;
        };
        let Operation::Select {
            from: Input::Step(read),
            ..
        } = &step.operation
        else {
            continue;
        };
        let Some(producer) = steps.get(usize::try_from(read.0).unwrap_or(usize::MAX)) else {
            continue;
        };
        if producer.scope != step.scope {
            continue;
        }
        if let Some(fused) = fuse(&producer.operation, &step.operation)
            && let Some(fusing) = steps.get_mut(place)
        {
            fusing.operation = fused;
        }
    }
    steps
}
