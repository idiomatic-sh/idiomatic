//! The `fusing` service: free functions named by verb over the filters and lookups of a plan that are read where they are found, instead of being made into tables first.

use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use ryo_relations::domain::relations::types::{Column, Relation};

use crate::domain::rewriting::types::StepIndex;
use ryo_values::domain::plan_parts::service::{
    column_place, remapped_comparison, rides_the_walk, view_source,
};
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// Physical demand propagation runs after ordinary column/pruning rewrites, then use counts are recomputed.
pub(crate) fn fuse_keyed_reaches(
    steps: Vec<PlanStep>,
    protected: &[ryo_values::domain::plan_parts::types::StepId],
) -> Vec<PlanStep> {
    super::types::demand::value::DemandClosure::new(&steps, protected).rewrite()
}

/// `consumer` as one step with `producer`, when both only filter and cut columns: the comparisons of
/// the producer, then the consumer's on the columns they name in the producer's input.
pub(crate) fn fuse(producer: &Operation, consumer: &Operation) -> Option<Operation> {
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
    let mut comparisons = first.clone();
    for comparison in second {
        comparisons.push(remapped_comparison(comparison, &|column| {
            view_source(cut, column)
        })?);
    }
    let kept = if last.is_empty() {
        cut.clone()
    } else {
        last.iter()
            .map(|column| view_source(cut, *column))
            .collect::<Option<Vec<_>>>()?
    };
    Some(Operation::Select {
        from: *from,
        comparisons,
        kept,
    })
}

/// The join at `place` with the right input read where its keys say, when that input is a filter of the
/// elements or of the edges of the syntax tree that rides the walk (`rides_the_walk`): the join asks for them by a node
/// (an element's id, an edge's parent or child) and every comparison of the filter is one no row can fail. They are not
/// made into a table. Any other filter stays a step of its own, the join's right input.
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
        || rides_the_walk(*relation, right_on, kept, comparisons).is_none()
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
        || view_source(kept, *domain_id)
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

/// The steps with each filter of the rows another filter kept made one filter of what the first reads,
/// so the rows between them are never built. The step it was fused from stays for any other reader;
/// the pruning of unused steps drops it when there is none.
#[must_use]
pub(crate) fn fuse_selects(mut steps: Vec<PlanStep>) -> Vec<PlanStep> {
    for place in 0..steps.len() {
        if let Some(fused) =
            fuse_join(&steps, StepIndex(place)).or_else(|| fuse_axis(&steps, StepIndex(place)))
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
