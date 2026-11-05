//! Execute the language schedule; alternatives never select a last-visited declaration as unique.
use crate::domain::{
    binding_observations::service::observe,
    binding_results::types::ReadContext,
    binding_schedule::service::schedule,
    binding_states::service::{clear as clear_environments, join, open},
    model::types::{
        BindingAction, BindingAnalysisIndex, BindingAnalysisState, BindingBranch, BindingPhaseKind,
        Declarations,
    },
};
use ryo::languages::{self, Integer, Position, Symbols, Tree};

/// Clear current-source observations while retaining every reusable buffer.
pub(crate) fn clear(index: &mut BindingAnalysisIndex) {
    index.state = BindingAnalysisState::Fresh;
    index.events.clear();
    index.reads.clear();
    index.uses.clear();
    index.shadows.clear();
    index.candidates.clear();
    index.mutations.clear();
    index.phases.clear();
    index.branches.clear();
    index.arms.clear();
    index.schedule.steps.clear();
    index.schedule.phases.clear();
    clear_environments(&mut index.environments);
}

/// Build one current-source index. No source positions are used as evaluation timestamps.
#[expect(
    clippy::too_many_lines,
    reason = "this function executes the single deterministic schedule state machine"
)]
pub(crate) fn build(
    tree: &Tree<'_>,
    model: &Declarations,
    symbols: &mut Symbols,
    index: &mut BindingAnalysisIndex,
) {
    clear(index);
    if languages::has_incomplete_syntax(tree.elements) {
        index.state = BindingAnalysisState::Incomplete;
        return;
    }
    schedule(tree, model, symbols, &mut index.schedule);
    crate::domain::binding_uncertainty::service::prepare(tree, model, index);
    let mut frame = None;
    let mut context = ReadContext::Value;
    for cursor in 0..index.schedule.steps.len() {
        let Some(step) = index.schedule.steps.get(cursor).copied() else {
            break;
        };
        match step.action {
            BindingAction::BeginPhase => {
                let task = index
                    .schedule
                    .phases
                    .iter()
                    .find(|task| task.root == step.phase);
                let parent = task.and_then(|task| index.phases.get(&task.parent).copied());
                if step.phase != Position(0) && parent.is_none() {
                    index.state = BindingAnalysisState::Incomplete;
                    return;
                }
                context = if task.is_some_and(|task| task.kind == BindingPhaseKind::Type) {
                    ReadContext::Type
                } else {
                    ReadContext::Value
                };
                frame = Some(open(&mut index.environments, parent, step.phase));
            }
            BindingAction::EndPhase => {
                if !index.branches.is_empty() {
                    index.state = BindingAnalysisState::Incomplete;
                    return;
                }
                if let Some(completed) = frame.take() {
                    let _previous_phase = index.phases.insert(step.phase, completed);
                }
            }
            BindingAction::BeginAlternatives => {
                let Some(target) = frame else {
                    index.state = BindingAnalysisState::Incomplete;
                    return;
                };
                index.branches.push(BindingBranch {
                    target,
                    start: index.arms.len(),
                });
                frame = Some(open(&mut index.environments, Some(target), step.phase));
            }
            BindingAction::NextAlternative => {
                let Some(branch) = index.branches.last().copied() else {
                    index.state = BindingAnalysisState::Incomplete;
                    return;
                };
                if let Some(completed) = frame {
                    index.arms.push(completed);
                }
                frame = Some(open(
                    &mut index.environments,
                    Some(branch.target),
                    step.phase,
                ));
            }
            BindingAction::EndAlternatives => {
                let Some(branch) = index.branches.pop() else {
                    index.state = BindingAnalysisState::Incomplete;
                    return;
                };
                if let Some(completed) = frame {
                    index.arms.push(completed);
                }
                let Some(arms) = index.arms.get(branch.start..) else {
                    index.state = BindingAnalysisState::Incomplete;
                    return;
                };
                join(&mut index.environments, branch.target, arms);
                index.arms.truncate(branch.start);
                frame = Some(branch.target);
            }
            BindingAction::Read
            | BindingAction::Bind
            | BindingAction::Annotation
            | BindingAction::Delete
            | BindingAction::Cleanup
            | BindingAction::Global
            | BindingAction::Nonlocal => {
                let Some(frame) = frame else {
                    index.state = BindingAnalysisState::Incomplete;
                    return;
                };
                let Ok(ordinal) = i64::try_from(cursor) else {
                    index.state = BindingAnalysisState::Incomplete;
                    return;
                };
                observe(tree, model, frame, step, context, Integer(ordinal), index);
            }
        }
    }
    index.state = BindingAnalysisState::Complete;
}

/// Compute once for the current source, preserving request-order independence across all four facts.
pub(crate) fn indexed<'a>(
    tree: &Tree<'_>,
    model: &'a Declarations,
    symbols: &mut Symbols,
) -> Option<std::cell::Ref<'a, BindingAnalysisIndex>> {
    if model.binding_analysis.borrow().state == BindingAnalysisState::Fresh {
        build(
            tree,
            model,
            symbols,
            &mut model.binding_analysis.borrow_mut(),
        );
    }
    let index = model.binding_analysis.borrow();
    (index.state == BindingAnalysisState::Complete).then_some(index)
}
