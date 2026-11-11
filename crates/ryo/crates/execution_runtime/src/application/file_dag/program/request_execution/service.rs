//! A request execution: its files, its project phase and its observation stage.
use crate::application::file_dag::frontiers::errors::FrontiersError;
use crate::application::file_dag::frontiers::service::PreparationPolicy;
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::program::facade_runtime;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::file_session::interface::ProgramFileSessionPort;
use crate::application::file_dag::program::file_session::service::FileSession;
use crate::application::file_dag::program::file_session::types::CaptureContext;
use crate::application::file_dag::program::file_session::types::EmittedRoot;
use crate::application::file_dag::program::file_session::types::FileEnd;
use crate::application::file_dag::program::file_session::types::FileInput;
use crate::application::file_dag::program::file_session::types::FileRules;
use crate::application::file_dag::program::file_session::types::FileRunInput;
use crate::application::file_dag::program::file_session::types::FileStop;
use crate::application::file_dag::program::file_session::types::KindSupport;
use crate::application::file_dag::program::file_session::types::RanRule;
use crate::application::file_dag::program::file_session::types::RuleAfter;
use crate::application::file_dag::program::file_session::types::RuleFile;
use crate::application::file_dag::program::file_session::types::SemanticSupport;
use crate::application::file_dag::program::file_session::types::SessionParts;
use crate::application::file_dag::program::file_session::types::SourceLanguage;
use crate::application::file_dag::program::file_session::types::SourceRows;
use crate::application::file_dag::program::observation_stage::service::ObservationStage;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::application::file_dag::program::project_phase::service::ProjectPhase;
use crate::application::file_dag::program::project_phase::types::FrontierDemand;
use crate::application::file_dag::program::project_phase::types::FrontierRequest;
use crate::application::file_dag::program::project_phase::types::ProjectInput;
use crate::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::FactsWorker;
use crate::outbound::facts::types::ResidentStability;
use crate::outbound::facts::types::WordsToKeep;
use ryo_execution_domain::domain::finish::types::Arena;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::{
    DiagnosticDelivery, EmitDemand, FileRuleIntent, RuleDemand,
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::CorpusInput;
use std::cell::RefCell;
use std::sync::Arc;
impl ProgramRequestExecutionPort for RequestExecution {
    fn observations<'a>(
        &'a self,
        arenas: &'a mut Vec<ryo_values::domain::stored_facts::types::ArenaBytes>,
    ) -> Result<ObservationStage<'a>, OperatingError> {
        let metadata = self
            .program
            .observation_metadata()
            .ok_or(OperatingError::NotRun)?;
        let phase = self.project_phase()?;
        let publications =
            crate::application::file_dag::observations::types::CrossPublications::new(
                metadata,
                &phase.publication,
                arenas,
            );
        Ok(ObservationStage {
            phase,
            publications,
        })
    }
    fn finish(self) {
        drop(self);
    }

    #[cfg(any(test, feature = "fixtures"))]
    fn open_file<'a, 'facts: 'a>(
        &'a self,
        input: FileInput<'a, 'facts>,
        symbols: &mut Symbols,
    ) -> Result<FileSession<'a, 'facts>, OperatingError> {
        self.open_session(input, symbols)
            .map(|parts| FileSession { parts })
    }
    fn run_file(
        &self,
        input: FileRunInput<'_>,
        symbols: &mut Symbols,
        capture: CaptureContext<'_>,
        facts: &dyn FactsWorker,
    ) -> FileRules {
        run_file(self, input, symbols, capture, facts)
    }
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError> {
        Ok(ProjectPhase {
            request: self,
            publication: std::rc::Rc::new(facade_runtime::service::publication_owner(
                &self.program,
                &self.services,
                Arc::clone(&self.digest),
            )?),
        })
    }
}
impl RequestExecution {
    /// Prepares the frontiers of one occurrence over its scope. Crate-private: the phase's `evaluate_occurrences` is its one
    /// caller, so no caller outside the engine acquires or holds a project population.
    pub(crate) fn prepare_frontiers(
        &self,
        input: FrontierRequest<'_>,
        symbols: &mut Symbols,
    ) -> Result<PreparedFrontiers, FrontiersError> {
        let policy = match input.demand {
            FrontierDemand::Diagnostic => PreparationPolicy::Diagnostic,
            FrontierDemand::Observations { ready } => {
                PreparationPolicy::Observations { ready: &ready.0 }
            }
        };
        self.frontiers.prepared(
            &self.program,
            input.rule,
            input.first,
            input.facts,
            input.project,
            input.project_symbols,
            symbols,
            &input.scope.required,
            policy,
        )
    }
    /// Opens one file's session: the engine's own route under `run_file`, sealed to the crate. It answers the session's crate-private
    /// parts, so widening it to `pub` fails the crate's own build (`private_interfaces`).
    pub(crate) fn open_session<'a, 'facts: 'a>(
        &'a self,
        input: FileInput<'a, 'facts>,
        symbols: &mut Symbols,
    ) -> Result<SessionParts<'a, 'facts>, OperatingError> {
        let mut vocabulary = match input.kinds {
            KindSupport::Prepared(vocabulary) => Some(vocabulary),
            KindSupport::Absent => None,
        };
        let vocabulary = ryo_execution_domain::domain::kind_sets::service::prepare_vocabulary(
            &mut vocabulary,
            input.elements,
            symbols,
            input.source,
        );
        let inner = facade_runtime::file_execution::service::open_file(
            &self.program,
            input.file,
            ScopePartials::File {
                evaluation: input.evaluation,
                publication: input.publication,
            },
            input.intents,
            Arc::clone(&self.services),
            input.source,
            if matches!(input.language, SourceLanguage::Recognized)
                && input.resident_stability == ResidentStability::Stable
            {
                ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible
            } else {
                ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Ineligible
            },
            symbols,
            match input.semantic {
                SemanticSupport::Supplied(lookup) => Some(lookup),
                SemanticSupport::Absent => None,
            },
            vocabulary,
        )?;
        Ok(SessionParts {
            inner,
            program: Arc::clone(&self.program),
            read: input.read,
            elements: input.elements,
            file: input.file,
        })
    }
    /// Sessions exist only inside the crate-private `ProjectPhase::evaluate`, one per declared root, which only
    /// `ProjectRootExecution::evaluate_occurrences` calls.
    pub(crate) fn open_project<'a>(
        &'a self,
        input: ProjectInput<'a>,
        symbols: &mut Symbols,
    ) -> Result<facade_runtime::project_execution::service::ProjectRun<'a>, OperatingError> {
        let frontiers = facade_runtime::service::ProjectFrontiers {
            publication: &input.scope.publication,
            ordinary: input.ordinary,
            bound_inputs: input.bound,
        };
        let inner = facade_runtime::project_execution::service::open_project(
            &self.program,
            input.scope.required.clone(),
            ScopePartials::Project {
                evaluation: input.evaluation,
                publication: input.publication,
            },
            frontiers,
            symbols,
            input.arenas,
            Arc::clone(&self.services),
        )?;
        Ok(inner)
    }
}

/// The rules of one file, one after another in the program's rule order, in one session over the worker's facts. A read of the
/// facts the engine refused is answered as the facts' own refusal, and stops the file where it happened.
fn run_file(
    request: &RequestExecution,
    input: FileRunInput<'_>,
    symbols: &mut Symbols,
    capture: CaptureContext<'_>,
    facts: &dyn FactsWorker,
) -> FileRules {
    let FileRunInput {
        language,
        file,
        evaluation,
        publication,
        intents,
        source,
        elements,
        semantic,
        kinds,
        first,
    } = input;
    let refused = RefCell::new(None);
    let read = |relation| {
        facts
            .rows(relation)
            .map(SourceRows::Stored)
            .map_err(|error| {
                *refused.borrow_mut() = Some(error);
                OperatingError::NotRead
            })
    };
    let CaptureContext {
        worker,
        arena: Arena(arena),
        places,
        words,
    } = capture;
    let mut rules = Vec::new();
    let opened = request.open_session(
        FileInput {
            language,
            resident_stability: facts.resident_stability(),
            file,
            evaluation,
            publication,
            intents,
            source,
            elements,
            read: &read,
            semantic,
            kinds,
        },
        symbols,
    );
    let mut session = match opened {
        Ok(parts) => FileSession { parts },
        Err(error) => {
            return FileRules {
                rules,
                end: FileEnd::Refused(FileStop::Operating(error)),
            };
        }
    };
    for rule in request.program.rule_order() {
        let Some(FileRuleIntent::Run {
            diagnostic,
            observations,
            ..
        }) = intents.get(rule.0)
        else {
            continue;
        };
        let emits = *diagnostic != DiagnosticDelivery::Disabled;
        let demand = RuleDemand {
            emit: if emits {
                EmitDemand::Enabled
            } else {
                EmitDemand::Disabled
            },
            observations: *observations,
        };
        let outcome = session.rule(rule, demand, symbols);
        let (emit, after) = match outcome {
            Err(error) => (EmittedRoot::Withheld, stopped(error, &refused)),
            Ok(_) if refused.borrow().is_some() => (
                EmittedRoot::Withheld,
                stopped(OperatingError::NotRead, &refused),
            ),
            Ok(outcome) => {
                let emit = if emits {
                    EmittedRoot::Delivered(outcome.emit)
                } else {
                    EmittedRoot::Withheld
                };
                let after = if matches!(emit, EmittedRoot::Delivered(RootOutcome::Failed(_))) {
                    RuleAfter::Unreached
                } else {
                    rest_of_rule(
                        &mut session,
                        RuleFile { rule, file, first },
                        symbols,
                        CaptureContext {
                            worker,
                            arena: Arena(&mut *arena),
                            places: &mut *places,
                            words: &mut *words,
                        },
                        facts,
                        &refused,
                    )
                    .unwrap_or_else(|error| stopped(error, &refused))
                };
                (emit, after)
            }
        };
        let ran = matches!(after, RuleAfter::Ran { .. });
        rules.push(RanRule { rule, emit, after });
        if !ran {
            return FileRules {
                rules,
                end: FileEnd::Unfinished,
            };
        }
    }
    let end = match session.finish() {
        Ok(finished) => FileEnd::Finished(finished.readiness),
        Err(error) => FileEnd::Refused(FileStop::Operating(error)),
    };
    FileRules { rules, end }
}

/// The stop of a rule: the facts' own refusal when a read of them was refused, else the engine's `error`.
fn stopped(error: OperatingError, refused: &RefCell<Option<FactsError>>) -> RuleAfter {
    RuleAfter::Stopped(
        refused
            .take()
            .map_or(FileStop::Operating(error), FileStop::Facts),
    )
}

/// What a rule does after its emitted root: its frontiers captured, each one captured kept in `facts` and committed, and its steps
/// reported to the observer, if any. A capture the engine could not make is answered as it is, for the caller to note.
fn rest_of_rule(
    session: &mut FileSession<'_, '_>,
    at: RuleFile<'_>,
    symbols: &mut Symbols,
    capture: CaptureContext<'_>,
    facts: &dyn FactsWorker,
    refused: &RefCell<Option<FactsError>>,
) -> Result<RuleAfter, OperatingError> {
    let first = *at.first.get(at.rule.0).ok_or(OperatingError::NotRun)?;
    let captured = session.capture_frontiers(at.rule, first, symbols, capture)?;
    if let Some(error) = refused.take() {
        return Ok(RuleAfter::Stopped(FileStop::Facts(error)));
    }
    let mut captures = Vec::with_capacity(captured.len());
    for each in captured {
        if let Ok(kept) = &each.outcome {
            if let Err(error) = facts.keep(
                kept.bucket,
                WordsToKeep {
                    words: &kept.words,
                    file: at.file,
                    proof: &kept.proof,
                },
            ) {
                return Ok(RuleAfter::Stopped(FileStop::Facts(error)));
            }
            session.commit_frontier(at.rule, kept.input)?;
        }
        captures.push(each);
    }
    session.observe(at.rule)?;
    Ok(RuleAfter::Ran { captures })
}

/// A request begins over the prepared program's kernels and shared services.
pub fn begin(
    program: &PreparedExecutionProgram,
    corpus: &[CorpusInput],
    observer: Option<Arc<dyn crate::outbound::observer::interface::ObserverPort>>,
) -> RequestExecution {
    RequestExecution {
        program: Arc::clone(&program.parts.inner),
        services: Arc::new(
            ProgramServices::new(
                Arc::clone(&program.parts.kernels),
                program.parts.derived.clone(),
                corpus,
            )
            .observed(observer),
        ),
        frontiers: crate::application::file_dag::frontiers::cache::service::FrontierCache::new(),
        digest: Arc::clone(&program.parts.digest),
    }
}
