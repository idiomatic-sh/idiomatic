//! One file's rules through the planned program: its facts read once, each rule's emitted node, named outputs and frontiers,
//! and the file finished with its named outputs' readiness.

use crate::application::file_dag::file_pass::service::{file_roots, fill_roots};
use crate::application::file_dag::program::facade_runtime::file_execution::service::{classify, readiness_of};
use crate::application::file_dag::fact_lookup::service::FactLookup;
use crate::application::file_dag::fact_lookup::types::FileWants;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::types::fact_sources::value::FactSources;
use crate::application::file_dag::file_pass::types::{CaptureAt, OutputCommit, OutputState, PassFile, PassInputs, PassProgram, PassRequest};
use crate::application::file_dag::file_pass::constants::SOURCE_FACTS_INCOMPLETE;
use crate::application::file_dag::program::file_session::types::{CaptureContext, EmittedRoot, FileEnd, FileRules, FileRunInput, FileStop, FinishedReadiness, Partiality, RanRule, RuleAfter};
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::outbound::facts::interface::FactsWorker;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::completion::types::ObservationAdmission;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::finish::types::Arena;
use ryo_execution_domain::domain::fused_program::service::{rule as fused_rule, serving};
use ryo_execution_domain::domain::fused_program::types::Serving;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::{DiagnosticDelivery, FileRuleIntent, ObservationDemand};
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::table_texts::types::Texts;
use ryo_values::domain::observations::service::{new_frontiers, set_frontier};
use ryo_values::domain::plan_parts::types::{Input, StepScope};
use ryo_values::domain::records::types::{ObservationReadinessCount, ObservationReadinessSlot, ProducerReadiness};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::rows::types::Table;
use std::cell::RefCell;
use std::sync::Arc;
use crate::application::file_dag::file_pass::service::nodes::service::{node_outcome, released};
use crate::application::file_dag::file_pass::service::captures::service::captured_rule;

/// One file's rules, one after another in the program's rule order, through the planned program: the file's facts read once,
/// each rule's emitted node run by the operators, its frontiers captured, kept and committed, its steps reported to the
/// observer, and the file finished with its named outputs' readiness. A read the facts refused stops the file as the facts' own
/// refusal; a failed emitted node ends the file.
pub(crate) fn run_rules(
    request: PassRequest<'_>,
    pass: &Result<PassProgram, OperatingError>,
    input: FileRunInput<'_>,
    symbols: &mut Symbols,
    capture: CaptureContext<'_>,
    facts: &mut dyn FactsWorker,
) -> FileRules {
    let (file, intents) = (input.file, input.intents);
    let wants = FileWants {
        active: input.active,
        said: input.said,
        recovery: input.recovery,
    };
    let lookup = match opened(input.sources, facts, wants, symbols, request.program.demanded()) {
        Ok(lookup) => lookup,
        Err(stop) => {
            return FileRules {
                rules: Vec::new(),
                end: FileEnd::Refused(stop),
                partials: Vec::new(),
            };
        }
    };
    let partials = lookup.partials().to_vec();
    let admitted = request
        .admission
        .clone()
        .and_then(|()| pass.as_ref().map_err(Clone::clone));
    let pass = match admitted {
        Ok(pass) => pass,
        Err(error) => {
            return FileRules {
                rules: Vec::new(),
                end: FileEnd::Refused(FileStop::Operating(error)),
                partials,
            };
        }
    };
    let evaluation = match input.answering {
        Partiality::Physical => partials.as_slice(),
        Partiality::Relaxed => &[][..],
    };
    let mut roots = file_roots(&pass.program);
    let (paths, program) = (request.paths, request.program);
    fill_roots(
        &mut roots,
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
            roots: &roots,
            refused: RefCell::new(None),
            unserved: Table::default(),
        },
        buffers: FileBuffers::default(),
        file,
        evaluation,
        publication: &partials,
        source: input.source,
        paths,
        outputs: vec![Vec::new(); program.rule_count().0],
    };
    let CaptureContext {
        worker,
        arena: Arena(arena),
        places,
        words,
    } = capture;
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
        // A rule that reads a feature no stage serves yet is refused where it would run, with the stage that serves it.
        if let Ok(Serving::NotServedUntil(stage)) = serving(&pass.program, rule) {
            rules.push(RanRule {
                rule,
                emit: EmittedRoot::Withheld,
                after: RuleAfter::Stopped(FileStop::Operating(OperatingError::NotServedUntil(stage))),
            });
            released(&mut at);
            return FileRules {
                rules,
                end: FileEnd::Unfinished,
                partials,
            };
        }
        let emits = diagnostic != DiagnosticDelivery::Disabled;
        let delivered = delivered_rule(&mut at, symbols, rule, diagnostic, observations);
        let refused = at.inputs.refused.take();
        let (emit, after) = match (delivered, refused) {
            (_, Some(error)) => (EmittedRoot::Withheld, RuleAfter::Stopped(FileStop::Facts(error))),
            (Err(error), None) => (EmittedRoot::Withheld, RuleAfter::Stopped(FileStop::Operating(error))),
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
            return FileRules {
                rules,
                end: FileEnd::Unfinished,
                partials,
            };
        }
    }
    let readiness = finished_readiness(&at);
    released(&mut at);
    FileRules {
        rules,
        end: FileEnd::Finished(readiness),
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
    let emit = if diagnostic == DiagnosticDelivery::Disabled {
        RootOutcome::NoQuery
    } else {
        let step = plan.emit.step.0 as usize;
        match plan.steps.get(step).map(|each| each.scope) {
            Some(phase) if phase != StepScope::PerFile => RootOutcome::Deferred { phase },
            _ => node_outcome(at, symbols, rule, nodes.emit.clone(), plan.emit.step),
        }
    };
    if observations == ObservationDemand::Required {
        let metadata = at.request.program.rule_metadata(rule)?;
        let mut states = vec![OutputState::default(); metadata.observations.len()];
        for (index, root) in metadata.observations.iter().enumerate() {
            if root.phase != StepScope::PerFile {
                continue;
            }
            let completion =
                CompletionPolicy::file_inputs(rule, at.file, at.evaluation, at.publication);
            let outcome = if completion.observation_admission(&root.reads) == ObservationAdmission::Ready {
                match plan.observations.get(index).map(|output| output.source) {
                    Some(Input::Step(id)) => {
                        let step = nodes
                            .steps
                            .get(id.0 as usize)
                            .ok_or(OperatingError::NotRun)
                            .and_then(Clone::clone);
                        node_outcome(at, symbols, rule, step, id)
                    }
                    Some(Input::Relation(_) | Input::BoundObservation(_)) | None => {
                        RootOutcome::Failed(OperatingError::NotRun)
                    }
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
    }
    Ok(emit)
}

/// The readiness of the request's named outputs after the open file, when the request names any.
pub(crate) fn finished_readiness(at: &PassFile<'_, '_, '_, '_>) -> FinishedReadiness {
    let Some(metadata) = at.request.program.observation_metadata() else {
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
            .and_then(|outputs| outputs.get(root.output.0 as usize))
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

/// Opens one file: every relation of `demanded` read, the active ones admitted. Any failure releases the rows written and stops
/// the file.
///
/// # Errors
///
/// `Reader` when a source refused, its rows released; `Facts` with the storage's own failure otherwise.
pub(crate) fn opened<'a, 's>(
    sources: FactSources<'s>,
    facts: &'a mut dyn FactsWorker,
    wants: FileWants<'a>,
    symbols: &mut Symbols,
    demanded: &[Relation],
) -> Result<FactLookup<'a, 's>, FileStop> {
    let mut lookup = FactLookup::new(sources, facts, wants);
    match lookup.ensure(symbols, demanded) {
        Ok(()) => Ok(lookup),
        Err(FactsError::Refused) => Err(FileStop::Reader),
        Err(other) => Err(FileStop::Facts(other)),
    }
}
