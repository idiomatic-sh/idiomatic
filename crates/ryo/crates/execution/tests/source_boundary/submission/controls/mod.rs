//! A control for every entry method: an incomplete, extra or undeclared root is refused.
use super::inventory::{
    ADMISSION, CAPTURES, DELIVERIES, DEMANDS, PROTOCOL, REASONS, REFUSALS, assert_inventory,
    covered, every_intent,
};
use super::routes::{file_route, project_route};
use super::witness::{ALL, Method, Witness, implements_every_entry_trait};
use crate::boundary_fixture::{corpus, demand, engine, prepared_rules};
use crate::file_intents::named_request;
use crate::project_phase::negative::fixtures::{
    Entry, Held, foreign_phase_outcome, prepare_for, with_held,
};
use ryo_execution::inbound::use_cases::execution::{
    interface::{
        ExecutionRequest, ExecutionUseCase, FileExecution, ObservationExecution,
        ObservationPublication, ProjectRootExecution, ProjectStage,
    },
    types::*,
};
use std::{collections::BTreeSet, path::Path};

/// What a method does with a root set: refuse an incomplete, extra or undeclared one, or
/// accept none at all (its inputs carry no rule, scope or intent to declare).
pub(super) enum Control {
    Refuses(fn()),
    AcceptsNoRoot(&'static str),
}
pub(super) fn control(method: Method) -> Control {
    match method {
        Method::UseCasePrepare => Control::Refuses(observations_need_a_named_program),
        Method::UseCaseBegin => Control::Refuses(file_session_undeclared_rule),
        Method::UseCasePrepareCompiled => Control::AcceptsNoRoot(
            "declares the rules of a request; no root exists before preparation",
        ),
        Method::RequestOpenFile => Control::Refuses(file_route),
        Method::RequestObservations => Control::Refuses(observations_need_a_named_program),
        Method::RequestPrepareFrontiers => Control::Refuses(frontiers_of_an_undeclared_rule),
        Method::RequestProjectPhase => Control::Refuses(a_scope_belongs_to_its_own_phase),
        Method::RootsEvaluate => Control::Refuses(evaluate_routes),
        Method::StageProjectScope => Control::Refuses(|| scope_of_an_undeclared_rule(Entry::Phase)),
        Method::ObservedProjectScope => {
            Control::Refuses(|| scope_of_an_undeclared_rule(Entry::Stage))
        }
        Method::ObservedReserve
        | Method::ObservedBindings
        | Method::ObservedBindingReadiness
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
pub(super) fn evaluate_routes() {
    project_route(Entry::Phase);
    project_route(Entry::Stage);
}
pub(super) fn observations_need_a_named_program() {
    let engine = engine();
    let corpus = corpus();
    let unnamed = prepared_rules(&engine, 2);
    let request = engine.begin(&unnamed, &corpus);
    let mut arenas = Vec::new();
    assert!(matches!(
        request.observations(&mut arenas),
        Err(OperatingError::NotRun)
    ));
    request.finish();
    let named = engine.prepare(named_request()).expect("named request");
    let request = engine.begin(&named, &corpus);
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
pub(super) fn a_scope_belongs_to_its_own_phase() {
    assert!(matches!(
        foreign_phase_outcome(&engine()),
        Err(OperatingError::NotRead)
    ));
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
    let request = engine.begin(&program, &corpus);
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
                resident_stability: facts::ResidentStability::Stable,
                file: ryo_relations::domain::relations::types::FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &intents,
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
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
                arena: &mut arena,
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
                .install(
                    RuleIndex(3),
                    PublicationSlot::Reserved(worker),
                    transfer,
                    vec![]
                )
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
                &[],
                &[ryo_values::domain::stored_facts::types::Bucket(0); 3],
                &[false; 3],
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
/// Methods of each entry trait in `interface.rs`, by the prefix of their `Method` variants.
const TRAITS: [(&str, &str); 7] = [
    ("ExecutionUseCase", "UseCase"),
    ("ExecutionRequest", "Request"),
    ("FileExecution", "File"),
    ("ProjectRootExecution", "Roots"),
    ("ProjectStage", "Stage"),
    ("ObservationExecution", "Observed"),
    ("ObservationPublication", "Publication"),
];
#[test]
pub(super) fn every_trait_method_has_its_own_variant() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let interface =
        std::fs::read_to_string(root.join("src/inbound/use_cases/execution/interface.rs")).unwrap();
    let witness =
        std::fs::read_to_string(root.join("tests/source_boundary/submission/witness/mod.rs"))
            .unwrap();
    for (name, prefix) in TRAITS {
        let start = interface
            .find(&format!("pub trait {name}"))
            .unwrap_or_else(|| panic!("{name} is declared in interface.rs"));
        let body = &interface[start..];
        let body = &body[..body.find("\n}\n").expect("a trait body ends")];
        let methods = body
            .lines()
            .filter(|line| line.starts_with("    fn "))
            .count();
        let variants = ALL
            .iter()
            .filter(|method| format!("{method:?}").starts_with(prefix))
            .count();
        assert_eq!(
            methods, variants,
            "{name} has {methods} methods and {variants} {prefix}* variants: one variant per method"
        );
    }
    for &method in ALL {
        let call = format!("witness(Method::{method:?})");
        assert_eq!(
            witness.matches(&call).count(),
            1,
            "{method:?} is witnessed by exactly one trait method"
        );
    }
    assert_eq!(
        witness.matches("witness(Method::").count(),
        ALL.len(),
        "every witnessed trait method names its own variant"
    );
}

#[test]
pub(super) fn every_entry_method_refuses_an_incomplete_extra_or_undeclared_root() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    implements_every_entry_trait(&Witness);
    assert_eq!(
        ALL.iter().collect::<BTreeSet<_>>().len(),
        ALL.len(),
        "each entry method has exactly one control"
    );
    assert_inventory(root, PROTOCOL, "DiagnosticDelivery", covered(&DELIVERIES));
    assert_inventory(root, PROTOCOL, "ObservationDemand", covered(&DEMANDS));
    assert_inventory(root, PROTOCOL, "CaptureIntent", covered(&CAPTURES));
    assert_inventory(root, PROTOCOL, "ApplicabilityRefusal", covered(&REFUSALS));
    assert_inventory(root, ADMISSION, "ApplicabilityReason", covered(&REASONS));
    assert_inventory(root, PROTOCOL, "FileRuleIntent", covered(&every_intent()));
    let (mut refused, mut exempt) = (0, 0);
    for &method in ALL {
        match control(method) {
            Control::Refuses(refuses) => {
                refuses();
                refused += 1;
            }
            Control::AcceptsNoRoot(reason) => {
                assert!(!reason.is_empty(), "{method:?} states why it takes no root");
                exempt += 1;
            }
        }
    }
    assert_eq!(
        refused + exempt,
        ALL.len(),
        "every entry method was classified"
    );
}
