//! A control for every entry method: an incomplete, extra or undeclared root is refused.
use ryo_execution_runtime::application::file_dag::observations::types::RuleSelection;
use super::inventory::{
    CAPTURES, DELIVERIES, DEMANDS, REASONS, REFUSALS, assert_inventory, capture_place,
    delivery_place, demand_place, every_intent, intent_place, reason_place, refusal_place,
};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use super::routes::{file_route, run_file_route};
use super::witness::{ALL, Method, Witness, implements_every_entry_trait};
use crate::boundary_fixture::{corpus, demand, engine, prepared_rules};
use crate::file_intents::named_request;
use crate::project_phase::negative::fixtures::{
    Entry, Held, NoFacts, prepare_for, with_held,
};
use ryo_execution::inbound::use_cases::execution::interface::OccurrenceNumber;
use ryo_execution::inbound::use_cases::execution::interface::FrontierDemand;
use ryo_execution::inbound::use_cases::execution::interface::PhaseOccurrence;
use ryo_execution::inbound::use_cases::execution::interface::PhaseOccurrences;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseStop;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileExecution;
use ryo_execution::inbound::use_cases::execution::interface::ObservationExecution;
use ryo_execution::inbound::use_cases::execution::interface::ObservationPublication;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution_runtime::application::file_dag::frontiers::errors::PreparationError;
use ryo_execution_runtime::application::file_dag::frontiers::types::PublicationSlot;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles;
use ryo_execution::inbound::use_cases::execution::interface::RefusedFiles;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use std::collections::BTreeSet;

/// What a method does with a root set: refuse an incomplete, extra or undeclared one, or
/// accept none at all (its inputs carry no rule, scope or intent to declare).
pub(super) enum Control {
    Refuses(fn()),
    AcceptsNoRoot(&'static str),
}
pub(super) fn control(method: Method) -> Control {
    match method {
        Method::UseCasePrepare => Control::Refuses(observations_need_a_named_program),
        Method::UseCaseExecution => Control::Refuses(file_session_undeclared_rule),
        Method::RequestOpenFile => Control::Refuses(file_route),
        Method::RequestRunFile => Control::Refuses(run_file_route),
        Method::RequestObservations => Control::Refuses(observations_need_a_named_program),
        Method::RequestPrepareFrontiers => Control::Refuses(frontiers_of_an_undeclared_rule),
        Method::RequestProjectPhase => Control::Refuses(each_phase_owns_its_publication),
        Method::RootsEvaluateOccurrences => Control::Refuses(occurrence_of_an_undeclared_rule),
        Method::StageProjectScope => Control::Refuses(|| scope_of_an_undeclared_rule(Entry::Phase)),
        Method::ObservedProjectScope => {
            Control::Refuses(|| scope_of_an_undeclared_rule(Entry::Stage))
        }
        Method::ObservedReserve
        | Method::ObservedBindings
        | Method::ObservedBindingReadiness
        | Method::ObservedFrontierReadiness
        | Method::ObservedFinish => Control::Refuses(observed_undeclared_rule),
        Method::FileEmitRows
        | Method::FileRule
        | Method::FileCaptureFrontiers
        | Method::FileCommitFrontier
        | Method::FileCounts => Control::Refuses(file_session_undeclared_rule),
        Method::FileReadinessCounts => Control::AcceptsNoRoot("reads counters"),
        Method::FileWalkCounts => Control::AcceptsNoRoot("reads counters"),
        Method::FileFinish => Control::AcceptsNoRoot("releases the file session"),
        Method::RequestFinish => Control::AcceptsNoRoot("drops the request"),
        Method::RootsReadinessCounts => Control::AcceptsNoRoot("reads counters"),
        Method::StageStateCounts => Control::AcceptsNoRoot("reads counters"),
        Method::StageFinish => Control::AcceptsNoRoot("releases the phase"),
        Method::PublicationInstall => Control::Refuses(install_undeclared_rule),
        Method::PublicationRestoreFrontiers => Control::Refuses(restore_inapplicable_per_rule),
        Method::ObservedArenas => Control::AcceptsNoRoot("reads published arenas"),
        Method::ObservedFinishStage => Control::AcceptsNoRoot("releases the stage"),
    }
}
pub(super) fn observations_need_a_named_program() {
    let engine = engine();
    let corpus = corpus();
    let unnamed = prepared_rules(&engine, 2);
    let request = ExecutionUseCaseImpl::new().execution(&unnamed, &corpus);
    let mut arenas = Vec::new();
    assert!(matches!(
        request.observations(&mut arenas),
        Err(OperatingError::NotRun)
    ));
    request.finish();
    let named = engine.prepare(named_request()).expect("named request");
    let request = ExecutionUseCaseImpl::new().execution(&named, &corpus);
    let mut arenas = Vec::new();
    request
        .observations(&mut arenas)
        .expect("a named program opens its stage")
        .finish_stage()
        .unwrap();
    request.finish();
}
pub(super) fn frontiers_of_an_undeclared_rule() {
    with_held(
        &engine(),
        Entry::Phase,
        3,
        |request, held, corpus, covered| {
            let scope = held.try_scope(RuleIndex(0), corpus, covered).unwrap();
            assert!(prepare_for(request, &scope, RuleIndex(0)).is_ok());
            assert!(matches!(
                prepare_for(request, &scope, RuleIndex(3)),
                Err(PreparationError::Metadata(OperatingError::NotRead))
            ));
        },
    );
}
/// A phase refuses the occurrence of a rule the program did not declare, and that refusal stays with the occurrence: the declared
/// occurrences beside it, a repeated rule included, are each evaluated and completed with their slot in declaration order, and the
/// phase is released once the call returns.
pub(super) fn occurrence_of_an_undeclared_rule() {
    with_held(&engine(), Entry::Phase, 3, |request, _, corpus, covered| {
        let inapplicable = vec![InapplicableFiles::default(); 4];
        let refused = vec![RefusedFiles::default(); 4];
        let project_symbols = ryo_language::domain::symbols::types::Symbols::default();
        let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
        let occurrence = |rule| PhaseOccurrence {
            rule,
            bound: &[],
            slot: PublicationSlot::Unused,
            demand: FrontierDemand::Diagnostic,
        };
        let phase = request.project_phase().expect("the phase opens");
        let ran = phase.evaluate_occurrences(
            PhaseOccurrences {
                occurrences: &[occurrence(RuleIndex(0)), occurrence(RuleIndex(0)), occurrence(RuleIndex(3))],
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
            |occurrence: OccurrenceNumber, _, slot, _| Ok((occurrence, slot)),
        );
        let ProjectBatch::Complete(occurrences) = ran else {
            panic!("no guard refused the batch");
        };
        assert!(matches!(
            occurrences.first(),
            Some(Ok((OccurrenceNumber(0), PublicationSlot::Unused)))
        ));
        assert!(matches!(
            occurrences.get(1),
            Some(Ok((OccurrenceNumber(1), PublicationSlot::Unused)))
        ));
        assert!(matches!(
            occurrences.get(2),
            Some(Err(PhaseStop::Operating(OperatingError::NotRun)))
        ));
        assert_eq!(occurrences.len(), 3);
        assert!(phase.finish().is_ok(), "the phase is released once its occurrences are completed");
    });
}
/// Each phase a request opens owns its own publication: a scope one phase still holds makes that phase's finish refuse, and leaves
/// another phase of the same request free to finish.
pub(super) fn each_phase_owns_its_publication() {
    with_held(&engine(), Entry::Phase, 3, |request, _, corpus, covered| {
        let held = request.project_phase().expect("the first phase opens");
        let other = request.project_phase().expect("the second phase opens");
        let scope = held
            .project_scope(RuleIndex(0), corpus, covered, &[], &InapplicableFiles::default())
            .expect("a declared rule has a scope");
        assert!(other.finish().is_ok(), "another phase's scope does not hold this phase's publication");
        assert!(
            matches!(held.finish(), Err(OperatingError::NotRead)),
            "an outstanding scope keeps its own phase's publication shared"
        );
        drop(scope);
    });
}
pub(super) fn scope_of_an_undeclared_rule(entry: Entry) {
    with_held(&engine(), entry, 3, |_, held, corpus, covered| {
        assert!(held.try_scope(RuleIndex(2), corpus, covered).is_ok());
        assert!(matches!(
            held.try_scope(RuleIndex(3), corpus, covered),
            Err(OperatingError::NotRun)
        ));
    });
}
/// Every file-session method that takes a RuleIndex refuses a rule the session did not declare.
pub(super) fn file_session_undeclared_rule() {
    let engine = engine();
    let program = prepared_rules(&engine, 2);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
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
    assert!(
        file.counts(RuleIndex(1)).is_ok(),
        "a declared rule is known"
    );
    assert!(file.counts(undeclared).is_err());
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
pub(super) fn observed_undeclared_rule() {
    with_held(&engine(), Entry::Stage, 3, |_, held, _, _| {
        let Held::Stage(stage) = held else {
            panic!("the stage entry holds an observation stage")
        };
        let undeclared = RuleIndex(3);
        assert!(stage.reserve(undeclared).is_err());
        assert!(stage.bindings(undeclared).is_err());
        assert!(stage.binding_readiness(undeclared).is_err());
        assert!(stage.frontier_readiness(undeclared).is_err());
        assert!(ObservationExecution::finish(stage, undeclared).is_err());
    });
}
/// A publication is installed only for a rule the program declared.
pub(super) fn install_undeclared_rule() {
    with_held(&engine(), Entry::Stage, 3, |_, held, _, _| {
        let Held::Stage(stage) = held else {
            panic!("the stage entry holds an observation stage")
        };
        let worker = ryo_relations::domain::relations::types::WorkerNumber(1);
        let transfer = ryo_values::domain::records::types::CrossPublicationTransfer {
            worker,
            arena: ryo_values::domain::stored_facts::types::ArenaBytes::default(),
            roots: vec![],
        };
        assert!(
            stage
                .install(RuleIndex(3), PublicationSlot::Reserved(worker), transfer, vec![])
                .is_err()
        );
    });
}
/// Restoration takes the inapplicable files of every rule or of none; any other count is refused.
pub(super) fn restore_inapplicable_per_rule() {
    with_held(&engine(), Entry::Stage, 3, |_, held, corpus, covered| {
        let Held::Stage(stage) = held else {
            panic!("the stage entry holds an observation stage")
        };
        let mut restore = |inapplicable: usize| {
            stage.restore_frontiers(
                &crate::project_phase::negative::fixtures::NoFileFrontiers,
                corpus,
                covered,
                &[],
                &vec![InapplicableFiles::default(); inapplicable],
                &vec![RefusedFiles::default(); 3],
                &[],
                &[ryo_values::domain::stored_facts::types::Bucket(0); 3],
                &[RuleSelection::Unselected; 3],
            )
        };
        for wrong in [1, 2, 4] {
            assert!(
                matches!(restore(wrong), Err(OperatingError::NotRun)),
                "{wrong} sets of inapplicable files for 3 rules"
            );
        }
        restore(0).expect("none for the whole run");
        restore(3).expect("one set per rule");
    });
}

mod census;
