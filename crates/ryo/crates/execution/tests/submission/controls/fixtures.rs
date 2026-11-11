//! The support of the `controls` cases and of the groups nested in it.
use super::super::inventory::fixtures::every_intent;
use super::super::routes::fixtures::{file_route, run_file_route};
use super::super::witness::fixtures::Method;
use crate::boundary_fixture::fixtures::{authored_request, corpus, demand, engine, prepared_rules};
use crate::file_intents::fixtures::named_request;
use crate::project_phase::negative::fixtures::{Entry, Held, with_held};
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles;
use ryo_execution::inbound::use_cases::execution::interface::OccurrenceNumber;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::PhaseStop;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramObservationStagePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::RefusedFiles;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::{
    BatchNumber, BatchOccurrence, ObservationBatches,
};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_negative::NoFacts;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::symbols::types::Symbols;

/// What a method does with a root set: refuse an incomplete, extra or undeclared one, or
/// accept none at all (its inputs carry no rule, scope or intent to declare).
pub(in super::super) enum Control {
    Refuses(fn()),
    AcceptsNoRoot(&'static str),
}

pub(in super::super) fn control(method: Method) -> Control {
    match method {
        Method::UseCasePrepare => Control::Refuses(prepare_refuses_an_incomplete_or_extra_rule_set),
        Method::UseCaseExecution => Control::Refuses(file_session_undeclared_rule),
        Method::RequestOpenFile => Control::Refuses(file_route),
        Method::RequestRunFile => Control::Refuses(run_file_route),
        Method::RequestObservations => Control::Refuses(observations_need_a_named_program),
        Method::RequestProjectPhase => {
            Control::AcceptsNoRoot("opens the phase; its roots are declared through evaluate_rules")
        }
        Method::RootsEvaluateRules => Control::Refuses(occurrence_of_an_undeclared_rule),
        Method::ObservedRunBatches => Control::Refuses(observed_undeclared_rule),
        Method::FileEmitRows
        | Method::FileRule
        | Method::FileCaptureFrontiers
        | Method::FileCommitFrontier => Control::Refuses(file_session_undeclared_rule),
        Method::FileFinish => Control::AcceptsNoRoot("releases the file session"),
        Method::RequestFinish => Control::AcceptsNoRoot("drops the request"),
        Method::StageFinish => Control::AcceptsNoRoot("releases the phase"),
    }
}

/// A program is prepared from a plan for each declared rule and no other: a rule set missing a planned rule, or with a rule no
/// plan answers, is refused.
pub(in super::super) fn prepare_refuses_an_incomplete_or_extra_rule_set() {
    let engine = engine();
    assert!(
        engine.prepare(authored_request(2)).is_ok(),
        "a complete declared set prepares"
    );
    let mut incomplete = authored_request(2);
    incomplete.rules.truncate(1);
    assert!(
        engine.prepare(incomplete).is_err(),
        "a planned rule missing from the set is refused"
    );
    let mut extra = authored_request(2);
    extra.rules.push(authored_request(3).rules.remove(2));
    assert!(
        engine.prepare(extra).is_err(),
        "a rule no plan answers is refused"
    );
}

pub(in super::super) fn observations_need_a_named_program() {
    let engine = engine();
    let corpus = corpus();
    let unnamed = prepared_rules(&engine, 2);
    let request = ExecutionUseCaseImpl::new().execution(&unnamed, &corpus, None);
    let mut arenas = Vec::new();
    assert!(matches!(
        request.observations(&mut arenas),
        Err(OperatingError::NotRun)
    ));
    request.finish();
    let named = engine.prepare(named_request()).expect("named request");
    let request = ExecutionUseCaseImpl::new().execution(&named, &corpus, None);
    let mut arenas = Vec::new();
    let stage = request
        .observations(&mut arenas)
        .expect("a named program opens its stage");
    stage
        .run_batches(
            stage_batches(&[], &[], &[], &corpus, &[], &[]),
            &mut Symbols::default(),
            |_| Ok::<(), OperatingError>(()),
            |_| Err(OperatingError::NotRun),
            |_| Ok(()),
            |_, _, _, _, _| Ok(()),
            |_, _| Ok(None),
        )
        .expect("a stage with no declared occurrence finishes");
    request.finish();
}

/// A phase refuses the occurrence of a rule the program did not declare, and that refusal stays with the occurrence: the declared
/// occurrences beside it, a repeated rule included, are each evaluated and completed with no reserved worker in declaration order,
/// and the phase is released once the call returns.
pub(in super::super) fn occurrence_of_an_undeclared_rule() {
    with_held(&engine(), Entry::Phase, 3, |_, held, corpus, covered| {
        let Held::Phase(phase) = held else {
            panic!("the phase entry holds a project phase")
        };
        let inapplicable = vec![InapplicableFiles::default(); 4];
        let refused = vec![RefusedFiles::default(); 4];
        let project_symbols = Symbols::default();
        let mut symbols = Symbols::default();
        let ran = phase.evaluate_rules(
            PhaseRules {
                rules: &[RuleIndex(0), RuleIndex(0), RuleIndex(3)],
                first: &[ryo_values::domain::stored_facts::types::Bucket(0); 4],
                corpus,
                covered,
                skipped: &[],
                inapplicable: &inapplicable,
                refused: &refused,
                facts: &NoFacts,
                project: &NoFacts,
                project_symbols: &project_symbols,
                arenas: &[],
            },
            &mut symbols,
            |_| Ok::<_, OperatingError>(()),
            |_| {
                Ok(PhasePartials {
                    evaluation: Partials::default(),
                    publication: Partials::default(),
                })
            },
            |_| Ok(()),
            |occurrence: OccurrenceNumber, _, _, worker, _| Ok((occurrence, worker)),
        );
        let ProjectBatch::Complete(occurrences) = ran else {
            panic!("no guard refused the batch");
        };
        assert!(matches!(
            occurrences.first(),
            Some(Ok((OccurrenceNumber(0), None)))
        ));
        assert!(matches!(
            occurrences.get(1),
            Some(Ok((OccurrenceNumber(1), None)))
        ));
        assert!(matches!(
            occurrences.get(2),
            Some(Err(PhaseStop::Operating(OperatingError::NotRun)))
        ));
        assert_eq!(occurrences.len(), 3);
        assert!(
            phase.finish().is_ok(),
            "the phase is released once its occurrences are completed"
        );
    });
}

/// Every file-session method that takes a RuleIndex refuses a rule the session did not declare.
pub(in super::super) fn file_session_undeclared_rule() {
    let engine = engine();
    let program = prepared_rules(&engine, 2);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| Err(OperatingError::NotRead);
    let intent = every_intent()
        .into_iter()
        .find(|intent| matches!(intent, FileRuleIntent::Run { .. }))
        .expect("a running intent exists");
    let intents = [intent, intent];
    let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: ResidentStability::Stable,
                file: ryo_relations::domain::relations::types::FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &intents,
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            &mut symbols,
        )
        .expect("a complete declared set opens");
    let undeclared = RuleIndex(2);
    assert!(file.rule(undeclared, demand(), &mut symbols).is_err());
    assert!(
        file.emit_rows(undeclared, &mut symbols, &mut |_| Ok(()))
            .is_err()
    );
    let (mut arena, mut places, mut words) = (vec![], vec![], Default::default());
    let bucket = ryo_values::domain::stored_facts::types::Bucket(0);
    assert!(
        file.capture_frontiers(
            undeclared,
            bucket,
            &mut symbols,
            CaptureContext {
                worker: ryo_relations::domain::relations::types::WorkerNumber(0),
                arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                places: &mut places,
                words: &mut words,
            },
        )
        .is_err()
    );
    assert!(
        file.commit_frontier(
            undeclared,
            ryo_values::domain::plan_parts::types::InputNumber(0)
        )
        .is_err()
    );
    let _ = file.finish();
    request.finish();
}

/// What one observation stage of the controls' program reads: `occurrences` over `rules`, with `inapplicable` files by rule.
pub(super) fn stage_batches<'a>(
    occurrences: &'a [BatchOccurrence],
    rules: &'a [RuleIndex],
    first: &'a [ryo_values::domain::stored_facts::types::Bucket],
    corpus: &'a [ryo_values::domain::files::types::CorpusInput],
    inapplicable: &'a [InapplicableFiles],
    refused: &'a [RefusedFiles],
) -> ObservationBatches<'a> {
    static PROJECT_SYMBOLS: std::sync::LazyLock<Symbols> =
        std::sync::LazyLock::new(Symbols::default);
    ObservationBatches {
        occurrences,
        readiness: &[],
        rules,
        first,
        corpus,
        covered: &[],
        skipped: &[],
        inapplicable,
        refused,
        facts: &NoFacts,
        project: &NoFacts,
        project_symbols: &PROJECT_SYMBOLS,
    }
}

/// A stage refuses an occurrence of a rule the program did not declare, and restoration takes the inapplicable files of every rule
/// or of none: any other count is refused before any occurrence runs.
pub(in super::super) fn observed_undeclared_rule() {
    let rules = [RuleIndex(0), RuleIndex(1), RuleIndex(2)];
    let first = [ryo_values::domain::stored_facts::types::Bucket(0); 3];
    let refused = vec![RefusedFiles::default(); 3];
    let run = |occurrences: &[BatchOccurrence], inapplicable: usize| {
        with_held(&engine(), Entry::Stage, 3, |_, held, corpus, _| {
            let Held::Stage(stage) = held else {
                panic!("the stage entry holds an observation stage")
            };
            let inapplicable = vec![InapplicableFiles::default(); inapplicable];
            stage.run_batches(
                stage_batches(occurrences, &rules, &first, corpus, &inapplicable, &refused),
                &mut Symbols::default(),
                |_| Ok::<(), OperatingError>(()),
                |_| {
                    Ok(PhasePartials {
                        evaluation: Partials::default(),
                        publication: Partials::default(),
                    })
                },
                |_| Ok(()),
                |_, _, _, _, _| Ok(()),
                |_, _| Ok(None),
            )
        })
    };
    assert!(
        matches!(
            run(
                &[BatchOccurrence {
                    batch: BatchNumber(0),
                    rule: RuleIndex(3)
                }],
                0
            ),
            Err(OperatingError::NotRun)
        ),
        "an occurrence of an undeclared rule"
    );
    for wrong in [1, 2, 4] {
        assert!(
            matches!(run(&[], wrong), Err(OperatingError::NotRun)),
            "{wrong} sets of inapplicable files for 3 rules"
        );
    }
    run(&[], 0).expect("none for the whole run");
    run(&[], 3).expect("one set per rule");
}
