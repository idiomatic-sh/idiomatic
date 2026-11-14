//! What a rule does after its emitted node: its frontiers captured, kept and committed, and its steps reported to the
//! observer.

use crate::application::file_dag::file_pass::service::roots_of;
use crate::application::file_dag::file_pass::types::{CaptureAt, NodeAnswer, OutputCommit, OutputState, PassFile};
use crate::application::file_dag::program::file_session::types::{CaptureContext, FileStop, RuleAfter};
use crate::outbound::facts::types::WordsToKeep;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::finish::service::escape_frame;
use ryo_execution_domain::domain::finish::types::{Arena, Leaves, Leaving};
use ryo_execution_domain::domain::frontier_capture::types::{CapturedFrontier, FrontierCapture};
use ryo_execution_domain::domain::frontier_coverage::service::file_frontier_coverage;
use ryo_execution_domain::domain::fused_program::service::rule as fused_rule;
use ryo_execution_domain::domain::fused_program::types::RuleNodes;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};
use ryo_execution_domain::domain::rule_intent::types::{CaptureIntent, FrontierPurpose};
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::physical_plan::types::NodeRows;
use ryo_operators::domain::tree_cells::service::native_form;
use ryo_values::domain::plan_parts::types::{Input, InputNumber, StepScope};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::{FrontierOrdinal, FrontierProof, ProducerRule};
use ryo_values::domain::stored_facts::service::frontier_bucket;
use ryo_values::domain::table_words::service::{encode_table, pack};
use ryo_values::domain::table_words::types::WordSlice;
use std::sync::Arc;
use crate::application::file_dag::file_pass::service::nodes::service::{node_rows, ran_node};

/// What a rule does after its emitted node: its frontiers captured, each one captured kept in the facts and committed, and its
/// steps reported to the observer, if any. A capture the engine could not make is answered as it is, for the caller to note.
pub(crate) fn captured_rule(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    capture: CaptureAt<'_>,
    context: CaptureContext<'_>,
) -> Result<RuleAfter, OperatingError> {
    let rule = capture.rule;
    let first = *capture.first.get(rule.0).ok_or(OperatingError::NotRun)?;
    if capture.intent == CaptureIntent::Disabled {
        return Err(OperatingError::NotRun);
    }
    let digest = at.request.program.rule_digest(rule)?;
    let frontiers = at.request.program.frontiers(rule)?;
    let nodes = fused_rule(&at.pass.program, rule)?.clone();
    let plan = Arc::clone(at.pass.plans.get(rule.0).ok_or(OperatingError::NotRun)?);
    let reads = at.pass.reads.get(rule.0).ok_or(OperatingError::NotRun)?.clone();
    let CaptureContext {
        worker,
        arena: Arena(arena),
        words,
        ..
    } = context;
    let mut captures = Vec::with_capacity(frontiers.len());
    for (index, source) in frontiers.iter().enumerate() {
        let input = InputNumber(index);
        if capture.intent == CaptureIntent::Observations
            && at.request.program.frontier_purpose(rule, input)? != FrontierPurpose::Observation
        {
            continue;
        }
        let step = match source {
            Input::Step(id) => Some(id.0 as usize),
            Input::Relation(_) | Input::BoundObservation(_) => None,
        };
        if step
            .and_then(|step| plan.steps.get(step))
            .is_some_and(|each| each.scope == StepScope::CrossFile)
        {
            continue;
        }
        let outcome = (|| {
            let step = step.ok_or(OperatingError::NotRun)?;
            let node = nodes.steps.get(step).ok_or(OperatingError::NotRun).and_then(Clone::clone);
            let NodeAnswer { mut table, coverage } = node_rows(at, symbols, rule, node, reads.get(step))?;
            let coverage = file_frontier_coverage(coverage, at.file)?;
            let frame_places = escape_frame(
                &mut table,
                Leaving {
                    symbols: &*symbols,
                    source: at.source,
                    file: at.file,
                    worker,
                    text: Leaves::WithText,
                },
                Arena(&mut *arena),
            )
            .map_err(|reason| OperatingError::ObservationIncomplete {
                reason: reason.to_string(),
            })?;
            words.0.clear();
            encode_table(&table, &frame_places, words);
            let packed = pack(WordSlice(&words.0));
            Ok(CapturedFrontier {
                input,
                bucket: frontier_bucket(first, input),
                proof: FrontierProof {
                    rule: ProducerRule(u32::try_from(rule.0).map_err(|_| OperatingError::ShapeMismatch)?),
                    input: FrontierOrdinal(u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?),
                    rule_digest: digest,
                    coverage,
                },
                words: packed,
            })
        })();
        captures.push(FrontierCapture { input, outcome });
    }
    if let Some(error) = at.inputs.refused.take() {
        return Ok(RuleAfter::Stopped(FileStop::Facts(error)));
    }
    for each in &captures {
        if let Ok(kept) = &each.outcome {
            if let Err(error) = at.inputs.lookup.keep(
                kept.bucket,
                WordsToKeep {
                    words: &kept.words,
                    file: at.file,
                    proof: &kept.proof,
                },
            ) {
                return Ok(RuleAfter::Stopped(FileStop::Facts(error)));
            }
            committed(at, rule, kept.input)?;
        }
    }
    observed(at, symbols, rule, &plan, &nodes)?;
    Ok(RuleAfter::Ran { captures })
}

/// Marks the named outputs `input` of `rule` feeds as committed in the open file.
pub(crate) fn committed(
    at: &mut PassFile<'_, '_, '_, '_>,
    rule: RuleIndex,
    input: InputNumber,
) -> Result<(), OperatingError> {
    for &output in at.request.program.file_frontier_outputs(rule, input)? {
        let held = at.outputs.get_mut(rule.0).ok_or(OperatingError::NotRun)?;
        if held.len() <= output.0 as usize {
            held.resize(output.0 as usize + 1, OutputState::default());
        }
        if let Some(state) = held.get_mut(output.0 as usize) {
            state.commit = OutputCommit::Committed;
        }
    }
    Ok(())
}

/// Reports the rows each step of `rule` was fed to the request's observer; without one nothing is counted.
pub(crate) fn observed(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    rule: RuleIndex,
    plan: &Plan,
    nodes: &RuleNodes,
) -> Result<(), OperatingError> {
    let Some(observer) = at.request.observer else {
        return Ok(());
    };
    let mut fed = Vec::with_capacity(plan.steps.len());
    for (step, node) in plan.steps.iter().zip(&nodes.steps) {
        let mut counts = [0_u64; 3];
        for (slot, input) in counts.iter_mut().zip(named(&inputs_of(&step.operation))) {
            *slot = match input {
                Input::Relation(relation) if native_form(*relation).is_some() => {
                    let node = node.clone()?;
                    roots_of(at.inputs.roots, node).0.len() as u64
                }
                Input::Relation(relation) => at
                    .inputs
                    .lookup
                    .columns(*relation)
                    .map_or(0, |view| view.columns.first().map_or(0, |column| column.0.len() as u64)),
                Input::Step(id) => {
                    let read = nodes.steps.get(id.0 as usize).ok_or(OperatingError::NotRun)?.clone()?;
                    ran_node(at, symbols, read)?;
                    match at.buffers.rows(read) {
                        NodeRows::Materialized(table) => table.count.0 as u64,
                        NodeRows::Absent => 0,
                    }
                }
                Input::BoundObservation(_) => 0,
            };
        }
        fed.push(RowsFed(counts));
    }
    observer.fed(rule, LogicalCounts { fed: fed.into_boxed_slice() })
}
