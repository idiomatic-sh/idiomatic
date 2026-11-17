//! What a rule does after its emitted node: its frontiers captured, kept and committed, and its steps reported to the
//! observer.

use crate::application::file_dag::file_pass::service::answering::service::step_fed;
use crate::application::file_dag::file_pass::service::nodes::service::node_rows;
use crate::application::file_dag::file_pass::service::reads::service::read;
use crate::application::file_dag::file_pass::types::{
    CaptureAt, NodeAnswer, OutputCommit, OutputState, PassFile,
};
use crate::application::file_dag::frontiers::service::reason;
use crate::application::file_dag::program::file_session::types::{
    CaptureContext, FileStop, RuleAfter,
};
use crate::application::file_dag::publications::service::bound::service::bound_coverage;
use crate::outbound::facts::types::WordsToKeep;
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::service::refusals::service::deferred_layout;
use ryo_execution_domain::domain::executable_program::service::rule as fused_rule;
use ryo_execution_domain::domain::executable_program::types::RuleNodes;
use ryo_execution_domain::domain::finish::service::escape_frame;
use ryo_execution_domain::domain::finish::types::{Arena, Leaves, Leaving};
use ryo_execution_domain::domain::frontier_capture::types::{CapturedFrontier, FrontierCapture};
use ryo_execution_domain::domain::frontier_coverage::service::file_frontier_coverage;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};
use ryo_execution_domain::domain::rule_intent::types::{CaptureIntent, FrontierPurpose};
use ryo_execution_domain::domain::table_model::service::materialize;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::physical_plan::types::PlanInputs as _;
use ryo_operators::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_operators::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_operators::domain::tree_cells::service::{native_form, tree_table, with_rows};
use ryo_relations::domain::catalog::service::scope_of;
use ryo_relations::domain::catalog::types::RowScope;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Input, InputNumber, StepScope};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::{FrontierOrdinal, FrontierProof, ProducerRule};
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::runs::types::{FileHandover, ParsedRows, RowsHanded, RuleNumber};
use ryo_values::domain::stored_facts::service::frontier_bucket;
use ryo_values::domain::table_words::service::{encode_table, pack};
use ryo_values::domain::table_words::types::WordSlice;
use std::sync::Arc;

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
    let CaptureContext {
        worker,
        arena: Arena(arena),
        words,
        ..
    } = context;
    let mut captures = Vec::with_capacity(frontiers.len());
    // The rows this file hands the rule's steps after the files, every input together, for an observer that asked.
    let mut handed = 0_u64;
    for (index, source) in frontiers.iter().enumerate() {
        let input = InputNumber(index);
        if capture.intent == CaptureIntent::Observations
            && at.request.program.frontier_purpose(rule, input)? != FrontierPurpose::Observation
        {
            continue;
        }
        // What the files hand over is the files': a step after the files, a project relation and a bound observation after
        // the files are handed over after the files, not kept here.
        let after_the_files = match *source {
            Input::Step(id) => plan
                .steps
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .is_some_and(|each| each.scope == StepScope::CrossFile),
            Input::Relation(relation) => scope_of(relation) == RowScope::Project,
            Input::BoundObservation(id) => plan
                .observation_inputs
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .is_none_or(|schema| schema.scope == StepScope::CrossFile),
        };
        if after_the_files {
            continue;
        }
        // A relation the file never read stops the rule here: nothing is handed over, as the interpreter stopped a rule whose
        // frontier named facts the file did not read.
        let relation = match *source {
            Input::Relation(relation) => match relation_rows(at, rule, relation) {
                Ok(answer) => Some(answer),
                Err(error) => return Ok(RuleAfter::Stopped(FileStop::Operating(error))),
            },
            Input::Step(_) | Input::BoundObservation(_) => None,
        };
        let outcome = (|| {
            let NodeAnswer {
                mut table,
                coverage,
            } = match *source {
                Input::Step(step) => {
                    let node = nodes
                        .steps
                        .get(usize::try_from(step.0).unwrap_or(usize::MAX))
                        .ok_or(OperatingError::NotRun)
                        .and_then(Clone::clone);
                    let deferred = node
                        .as_ref()
                        .is_ok_and(|node| deferred_layout(&at.pass.program, *node).is_some());
                    let mut answer = node_rows(at, symbols, rule, node, step)?;
                    // A deferred refusal holding no row is handed over with no column: the step after the files that reads it
                    // reads none, as a reader in the file does (`NodeMove::Empty`).
                    if deferred && answer.table.count.0 == 0 {
                        answer.table.kinds.clear();
                    }
                    answer
                }
                Input::Relation(_) => relation.ok_or(OperatingError::NotRead)?,
                Input::BoundObservation(id) => {
                    match at.inputs.observation(BoundObservationPlace::of(
                        RuleOrdinal(
                            u32::try_from(rule.0).map_err(|_| OperatingError::ShapeMismatch)?,
                        ),
                        id,
                    )) {
                        HeldRows::Held(table) => NodeAnswer {
                            table: table.clone(),
                            coverage: bound_coverage(&at.inputs.bound, id)
                                .cloned()
                                .unwrap_or(CoverageState::Complete),
                        },
                        HeldRows::Withheld(token) => {
                            return Err(reason(&at.inputs.withholdings, token));
                        }
                    }
                }
            };
            let coverage = file_frontier_coverage(coverage, at.file)?;
            let rows = table.count;
            handed = handed.saturating_add(u64::try_from(rows.0).unwrap_or(u64::MAX));
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
                    rule: ProducerRule(
                        u32::try_from(rule.0).map_err(|_| OperatingError::ShapeMismatch)?,
                    ),
                    input: FrontierOrdinal(
                        u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?,
                    ),
                    rule_digest: digest,
                    coverage,
                },
                words: packed,
                rows,
            })
        })();
        captures.push(FrontierCapture { input, outcome });
    }
    if let Some(observer) = at.request.observer
        && handed != 0
    {
        observer.handed(FileHandover {
            file: at.file,
            rule: RuleNumber(u32::try_from(rule.0).map_err(|_| OperatingError::ShapeMismatch)?),
            handed: RowsHanded(handed),
            parsed: ParsedRows(
                u64::try_from(at.inputs.lookup.tree().kind.len()).unwrap_or(u64::MAX),
            ),
        })?;
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
    observed(at, rule, &plan, &nodes)?;
    Ok(RuleAfter::Ran { captures })
}

/// A relation's rows in the open file, as the file's sources wrote them or, for a tree relation, as the open file's tree has
/// them at every one of its tree nodes; and their coverage as the file's partiality says.
fn relation_rows(
    at: &PassFile<'_, '_, '_, '_>,
    rule: RuleIndex,
    relation: Relation,
) -> Result<NodeAnswer, OperatingError> {
    let table = match native_form(relation) {
        Some(form) => tree_table(
            form,
            kinds(relation),
            at.inputs.lookup.tree(),
            with_rows(form, &at.inputs.roots.all),
        ),
        None => {
            // The file's one read of the relation, shared with its readers.
            let view = read(&at.inputs.reads, at.inputs.lookup, relation)
                .map_err(|_| OperatingError::NotRead)?;
            let mut table = Table::default();
            materialize(&Rows::Stored(view), &mut table, &mut Vec::new());
            table
        }
    };
    let completion = CompletionPolicy::file_inputs(rule, at.file, at.evaluation, at.publication);
    Ok(NodeAnswer {
        table,
        coverage: completion.relation_coverage(relation),
    })
}

/// Marks the named outputs `input` of `rule` feeds as committed in the open file.
pub(crate) fn committed(
    at: &mut PassFile<'_, '_, '_, '_>,
    rule: RuleIndex,
    input: InputNumber,
) -> Result<(), OperatingError> {
    for &output in at.request.program.file_frontier_outputs(rule, input)? {
        let held = at.outputs.get_mut(rule.0).ok_or(OperatingError::NotRun)?;
        if held.len() <= usize::try_from(output.0).unwrap_or(usize::MAX) {
            held.resize(
                usize::try_from(output.0).unwrap_or(usize::MAX) + 1,
                OutputState::default(),
            );
        }
        if let Some(state) = held.get_mut(usize::try_from(output.0).unwrap_or(usize::MAX)) {
            state.commit = OutputCommit::Committed;
        }
    }
    Ok(())
}

/// Reports the rows each step of `rule` was fed to the request's observer, as its node's kernels read them in the open file;
/// without an observer nothing is counted.
pub(crate) fn observed(
    at: &PassFile<'_, '_, '_, '_>,
    rule: RuleIndex,
    plan: &Plan,
    nodes: &RuleNodes,
) -> Result<(), OperatingError> {
    let Some(observer) = at.request.observer else {
        return Ok(());
    };
    // What each step's node's kernels read in the open file, when it ran: a step after the files is fed after the files, and a
    // step whose node never ran (no root reached it, or it answered a refusal without running its kernel) was fed none.
    let fed: Vec<RowsFed> = plan
        .steps
        .iter()
        .zip(&nodes.steps)
        .map(|(step, node)| match step.scope {
            StepScope::PerFile => step_fed(&at.buffers, step, node),
            StepScope::CrossFile => RowsFed::default(),
        })
        .collect();
    observer.fed(
        rule,
        LogicalCounts {
            fed: fed.into_boxed_slice(),
        },
    )
}
