//! One file's rules through the planned program: its facts read once, each rule's emitted node, named outputs and frontiers,
//! and the file finished with its named outputs' readiness.
use crate::application::file_dag::fact_lookup::service::FactLookup;
use crate::application::file_dag::file_pass::constants::SOURCE_FACTS_INCOMPLETE;
use crate::application::file_dag::file_pass::service::captures::service::captured_rule;
use crate::application::file_dag::file_pass::service::inputs::service::{
    hosted_in_file, in_file_bound,
};
use crate::application::file_dag::file_pass::service::nodes::service::{
    node_outcome, node_rows, released,
};
use crate::application::file_dag::file_pass::service::{fill_roots, given_back};
use crate::application::file_dag::file_pass::types::{
    CaptureAt, NodeAnswer, OpenedFile, OutputCommit, OutputState, PassFile, PassInputs,
    PassProgram, PassRequest, PassScratch,
};
use crate::application::file_dag::frontiers::types::Withholdings;
use crate::application::file_dag::program::file_session::types::{
    CaptureContext, EmittedRoot, FileEnd, FileRules, FileStop, FinishedReadiness, Partiality,
    RanRule, RuleAfter, SemanticSupport,
};
use crate::application::file_dag::program::rule_outcome::service::{classify, readiness_of};
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::application::file_dag::publications::types::BoundTables;
use ryo_execution_domain::domain::completion::types::ObservationAdmission;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::service::rule as fused_rule;
use ryo_execution_domain::domain::finish::types::Arena;
use ryo_execution_domain::domain::frontier_coverage::service::affected_files;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::{
    DiagnosticDelivery, FileRuleIntent, ObservationDemand,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::table_texts::types::Texts;
use ryo_values::domain::observations::service::{new_frontiers, set_frontier};
use ryo_values::domain::plan_parts::types::{Input, StepScope};
use ryo_values::domain::records::types::{
    ObservationReadinessCount, ObservationReadinessSlot, ProducerReadiness,
};
use std::cell::RefCell;
use std::sync::Arc;

/// One file's rules, one after another in the program's rule order, through the planned program over the file's facts, which the
/// caller opened (`fact_lookup`'s `opened`) and read once, in `scratch`, which the caller took and the run gives back:
/// each rule's emitted node run by the operators, its frontiers captured, kept and committed, its steps reported to the
/// observer, and the file finished with its named outputs' readiness. A read the facts refused stops the file as the facts' own
/// refusal; a failed emitted node ends the file.
pub(crate) fn run_rules(
    request: PassRequest<'_>,
    pass: &PassProgram,
    input: OpenedFile<'_>,
    opened: Result<FactLookup<'_, '_>, FileStop>,
    symbols: &mut Symbols,
    capture: CaptureContext<'_>,
    mut scratch: PassScratch,
) -> FileRules {
    let (file, intents) = (input.file, input.intents);
    let lookup = match opened {
        Ok(lookup) => lookup,
        Err(stop) => {
            given_back(pass, scratch);
            return FileRules {
                rules: Vec::new(),
                end: FileEnd::Refused(stop),
                partials: Vec::new(),
            };
        }
    };
    let partials = lookup.partials().to_vec();
    if let Err(error) = request.admission.clone() {
        given_back(pass, scratch);
        return FileRules {
            rules: Vec::new(),
            end: FileEnd::Refused(FileStop::Operating(error)),
            partials,
        };
    }
    let evaluation = match input.answering {
        Partiality::Physical => partials.as_slice(),
        Partiality::Relaxed => &[][..],
    };
    // What the request's last file left: its buffers, roots and outputs, grown once to the program and only refilled here.
    let (paths, program) = (request.paths, request.program);
    fill_roots(
        &mut scratch.roots,
        lookup.tree(),
        &Texts {
            symbols,
            source: input.source,
            paths,
            arenas: &[],
        },
        file,
    );
    let mut at = PassFile {
        request,
        pass,
        inputs: PassInputs {
            lookup: &lookup,
            roots: &scratch.roots,
            refused: RefCell::new(None),
            bound: BoundTables::default(),
            withholdings: Withholdings::default(),
            hosted: Vec::new(),
        },
        buffers: std::mem::take(&mut scratch.buffers),
        file,
        evaluation,
        publication: &partials,
        source: input.source,
        paths,
        semantic: match input.semantic {
            SemanticSupport::Supplied(lookup) => Some(lookup),
            SemanticSupport::Absent => None,
        },
        refusals: Vec::new(),
        outputs: std::mem::take(&mut scratch.outputs),
        answers: std::mem::take(&mut scratch.answers),
    };
    for outputs in &mut at.outputs {
        outputs.clear();
    }
    at.outputs.resize_with(program.rule_count().0, Vec::new);
    for answers in &mut at.answers {
        answers.clear();
    }
    at.answers.resize_with(program.rule_count().0, Vec::new);
    let CaptureContext {
        worker,
        arena: Arena(arena),
        places,
        words,
    } = capture;
    let ran = (|| {
        let mut rules = Vec::new();
        for rule in program.rule_order() {
            let Some(FileRuleIntent::Run {
                diagnostic,
                observations,
                capture: capture_intent,
            }) = intents.get(rule.0).copied()
            else {
                continue;
            };
            let emits = diagnostic != DiagnosticDelivery::Disabled;
            let delivered = delivered_rule(&mut at, symbols, rule, diagnostic, observations);
            let refused = at.inputs.refused.take();
            let (emit, after) = match (delivered, refused) {
                (_, Some(error)) => (
                    EmittedRoot::Withheld,
                    RuleAfter::Stopped(FileStop::Facts(error)),
                ),
                (Err(error), None) => (
                    EmittedRoot::Withheld,
                    RuleAfter::Stopped(FileStop::Operating(error)),
                ),
                (Ok(outcome), None) => {
                    let emit = if emits {
                        EmittedRoot::Delivered(outcome)
                    } else {
                        EmittedRoot::Withheld
                    };
                    let after = if matches!(emit, EmittedRoot::Delivered(RootOutcome::Failed(_))) {
                        RuleAfter::Unreached
                    } else {
                        let captured = captured_rule(
                            &mut at,
                            symbols,
                            CaptureAt {
                                rule,
                                first: input.first,
                                intent: capture_intent,
                            },
                            CaptureContext {
                                worker,
                                arena: Arena(&mut *arena),
                                places: &mut *places,
                                words: &mut *words,
                            },
                        );
                        match (captured, at.inputs.refused.take()) {
                            (_, Some(error)) => RuleAfter::Stopped(FileStop::Facts(error)),
                            (Err(error), None) => RuleAfter::Stopped(FileStop::Operating(error)),
                            (Ok(after), None) => after,
                        }
                    };
                    (emit, after)
                }
            };
            let ran = matches!(after, RuleAfter::Ran { .. });
            rules.push(RanRule { rule, emit, after });
            if !ran {
                released(&mut at);
                return (rules, FileEnd::Unfinished);
            }
        }
        let readiness = finished_readiness(&at);
        released(&mut at);
        (rules, FileEnd::Finished(readiness))
    })();
    let PassFile {
        buffers,
        outputs,
        answers,
        ..
    } = at;
    scratch.buffers = buffers;
    scratch.outputs = outputs;
    scratch.answers = answers;
    given_back(pass, scratch);
    let (rules, end) = ran;
    FileRules {
        rules,
        end,
        partials,
    }
}

/// The delivery of `rule`'s emitted node and, when demanded, its named outputs' readiness. A forwarded delivery is not this
/// entry point's.
pub(crate) fn delivered_rule(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    rule: RuleIndex,
    diagnostic: DiagnosticDelivery,
    observations: ObservationDemand,
) -> Result<RootOutcome, OperatingError> {
    if diagnostic == DiagnosticDelivery::Forwarded {
        return Err(OperatingError::NotRun);
    }
    let nodes = fused_rule(&at.pass.program, rule)?;
    let plan = at.pass.plans.get(rule.0).ok_or(OperatingError::NotRun)?;
    at.inputs.bound = in_file_bound(at, rule)?;
    hosted_in_file(at, symbols, plan);
    let emit = if diagnostic == DiagnosticDelivery::Disabled {
        RootOutcome::NoQuery
    } else {
        let step = usize::try_from(plan.emit.step.0).unwrap_or(usize::MAX);
        match plan.steps.get(step).map(|each| each.scope) {
            Some(phase) if phase != StepScope::PerFile => RootOutcome::Deferred { phase },
            _ => node_outcome(at, symbols, rule, nodes.emit.clone(), plan.emit.step),
        }
    };
    if observations == ObservationDemand::Required {
        let stated = at.request.program.rule_plan(rule)?;
        let mut states = vec![OutputState::default(); stated.roots.len()];
        let mut answers = vec![None; stated.roots.len()];
        for (index, root) in stated.roots.iter().enumerate() {
            if root.phase != StepScope::PerFile {
                continue;
            }
            let completion =
                CompletionPolicy::file_inputs(rule, at.file, at.evaluation, at.publication);
            let outcome =
                if completion.observation_admission(&root.reads) == ObservationAdmission::Ready {
                    let source = plan.observations.get(index).map(|output| output.source);
                    match output_rows(at, symbols, rule, source) {
                        Ok(answer) => {
                            let outcome = answered(&answer);
                            if let Some(kept) = answers.get_mut(index) {
                                *kept = Some(answer);
                            }
                            outcome
                        }
                        Err(error) => classify(error),
                    }
                } else {
                    classify(OperatingError::ObservationIncomplete {
                        reason: SOURCE_FACTS_INCOMPLETE.into(),
                    })
                };
            if let Some(state) = states.get_mut(index) {
                state.readiness = readiness_of(&outcome);
            }
        }
        if let Some(held) = at.outputs.get_mut(rule.0) {
            *held = states;
        }
        if let Some(held) = at.answers.get_mut(rule.0) {
            *held = answers;
        }
    }
    Ok(emit)
}

/// The rows of a rule's named output `source` in the open file, with their coverage.
fn output_rows(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    rule: RuleIndex,
    source: Option<Input>,
) -> Result<NodeAnswer, OperatingError> {
    match source {
        Some(Input::Step(id)) => {
            let step = fused_rule(&at.pass.program, rule)?
                .steps
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .ok_or(OperatingError::NotRun)
                .and_then(Clone::clone);
            node_rows(at, symbols, rule, step, id)
        }
        Some(Input::Relation(_) | Input::BoundObservation(_)) | None => Err(OperatingError::NotRun),
    }
}

/// A named output's answer from its rows and their coverage.
fn answered(answer: &NodeAnswer) -> RootOutcome {
    match affected_files(&answer.coverage) {
        None => RootOutcome::Complete(answer.table.clone()),
        Some(affected) => RootOutcome::Partial {
            table: answer.table.clone(),
            affected,
        },
    }
}

/// The readiness of the request's named outputs after the open file, when the request names any.
pub(crate) fn finished_readiness(at: &PassFile<'_, '_, '_, '_>) -> FinishedReadiness {
    let Some(metadata) = at.request.program.observation_layout() else {
        return FinishedReadiness::Unobserved;
    };
    if metadata.frontiers.is_empty() {
        return FinishedReadiness::Unobserved;
    }
    let Ok(count) = u32::try_from(metadata.frontiers.len()) else {
        return FinishedReadiness::Unobserved;
    };
    let mut map = new_frontiers(metadata.layout, ObservationReadinessCount(count));
    for (slot, root) in metadata.frontiers.iter().enumerate() {
        let Some(state) = at
            .outputs
            .get(root.producer.0)
            .and_then(|outputs| outputs.get(usize::try_from(root.output.0).unwrap_or(usize::MAX)))
        else {
            continue;
        };
        let readiness = match &state.readiness {
            Some(ProducerReadiness::Complete) if state.commit == OutputCommit::Committed => {
                ProducerReadiness::Complete
            }
            Some(ProducerReadiness::Complete) | None => continue,
            Some(other) => other.clone(),
        };
        let Ok(position) = u32::try_from(slot) else {
            return FinishedReadiness::Unobserved;
        };
        if set_frontier(&mut map, ObservationReadinessSlot(position), readiness).is_err() {
            return FinishedReadiness::Unobserved;
        }
    }
    FinishedReadiness::Observed(Arc::new(map))
}
