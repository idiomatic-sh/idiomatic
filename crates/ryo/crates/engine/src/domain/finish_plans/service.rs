//! The `finish_plans` service: what a plan's steps read, and who reads each.

use super::arguments::literals::types::{ArgumentValue, UnlistedArgument};
use super::arguments::types::StepNumber;
use super::constants::{
    DIRECTION_ANCESTOR, DIRECTION_CHILD, DIRECTION_DEEP, DIRECTION_DESCENDANT,
    DIRECTION_NEAREST_ANCESTOR, DIRECTION_NEXT_SIBLING, DIRECTION_PARENT,
    DIRECTION_PREVIOUS_SIBLING,
};
use super::types::FinishStep;
use ryo_values::domain::plan_parts::types::Direction;

/// The steps a step reads, in the order its sources or relation arguments are written; a step read twice is named twice.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::joins::types::UnionArguments;
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::service::inputs;
/// use ryo_engine::domain::finish_plans::types::FinishStep;
///
/// let union = FinishStep::Union(UnionArguments { left: StepNumber(0), right: StepNumber(1) });
/// assert_eq!(inputs(&union), vec![StepNumber(0), StepNumber(1)]);
/// ```
#[must_use]
pub fn inputs(step: &FinishStep) -> Vec<StepNumber> {
    match step {
        FinishStep::Base(_) | FinishStep::Rows(_) => Vec::new(),
        FinishStep::Select(select) => select.sources.iter().map(|source| source.step).collect(),
        FinishStep::Union(union) => vec![union.left, union.right],
        FinishStep::Antijoin(antijoin) => vec![antijoin.left, antijoin.right],
        FinishStep::AsOf(as_of) => vec![as_of.left, as_of.right],
        FinishStep::OrderedMatch(matched) => vec![matched.left, matched.right],
        FinishStep::Axis(axis) => vec![
            axis.arguments.from,
            axis.arguments.relation,
            axis.arguments.domain,
        ],
        FinishStep::Closure(edges) | FinishStep::Follow(edges) => vec![edges.edges],
        FinishStep::OrderedDfs(walk) => vec![walk.from, walk.edges],
        FinishStep::GroupCount(count) => vec![count.from],
        FinishStep::GroupMin(order) | FinishStep::GroupMax(order) => vec![order.from],
        FinishStep::GroupOrdinal(ordinal) => vec![ordinal.from],
        FinishStep::Project(project) => vec![project.from],
        FinishStep::Extend(extend) => vec![extend.from],
        FinishStep::SpanBounds(bounds) => vec![bounds.from],
        FinishStep::SpanBetween(between) => vec![between.from],
        FinishStep::Replace(replace) => vec![replace.from],
        FinishStep::SourceMatches(matches) => vec![matches.from],
        FinishStep::Capture(capture) => vec![capture.from],
        FinishStep::FitsRendered(fits) => vec![fits.from],
        FinishStep::RenderSyntax(render) => vec![render.from],
        FinishStep::Prefixes(prefixes) => vec![prefixes.from],
        FinishStep::SubtreeClass(classes) => {
            vec![classes.from, classes.relation, classes.labels, classes.leaves]
        }
        FinishStep::Unlisted(unlisted) => unlisted
            .arguments
            .iter()
            .filter_map(|argument: &UnlistedArgument| match argument.value {
                ArgumentValue::Step(step) => Some(step),
                ArgumentValue::Constant(_) => None,
            })
            .collect(),
    }
}

/// Who reads each step: for each step, by its number, the steps that read it, each once, in plan order.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::joins::types::UnionArguments;
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::service::readers;
/// use ryo_engine::domain::finish_plans::types::FinishStep;
/// use ryo_values::domain::descriptions::types::RelationName;
///
/// let steps = vec![
///     FinishStep::Base(RelationName("syntax-tree".into())),
///     FinishStep::Union(UnionArguments { left: StepNumber(0), right: StepNumber(0) }),
/// ];
/// assert_eq!(readers(&steps), vec![vec![StepNumber(1)], Vec::new()]);
/// ```
#[must_use]
pub fn readers(steps: &[FinishStep]) -> Vec<Vec<StepNumber>> {
    let mut read_by: Vec<Vec<StepNumber>> = vec![Vec::new(); steps.len()];
    for (place, step) in steps.iter().enumerate() {
        let reader = StepNumber(place);
        for input in inputs(step) {
            if let Some(list) = read_by.get_mut(input.0)
                && list.last() != Some(&reader)
            {
                list.push(reader);
            }
        }
    }
    read_by
}

/// How the plan format writes an axis direction.
pub(crate) fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Parent => DIRECTION_PARENT,
        Direction::Child => DIRECTION_CHILD,
        Direction::Ancestor => DIRECTION_ANCESTOR,
        Direction::Descendant => DIRECTION_DESCENDANT,
        Direction::NearestAncestor => DIRECTION_NEAREST_ANCESTOR,
        Direction::Deep => DIRECTION_DEEP,
        Direction::PreviousSibling => DIRECTION_PREVIOUS_SIBLING,
        Direction::NextSibling => DIRECTION_NEXT_SIBLING,
    }
}
