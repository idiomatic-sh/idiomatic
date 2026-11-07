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
use ryo_execution::application::file_dag::program::request_execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::application::file_dag::program::file_session::interface::FileExecution;
use ryo_execution::application::file_dag::program::observation_stage::interface::ObservationExecution;
use ryo_execution::application::file_dag::program::observation_stage::interface::ObservationPublication;
use ryo_execution::application::file_dag::program::project_phase::interface::ProjectRootExecution;
use ryo_execution::application::file_dag::program::project_phase::interface::ProjectStage;
use ryo_execution::application::file_dag::frontiers::errors::PreparationError;
use ryo_execution::application::file_dag::frontiers::types::PreparedFrontiers;
use ryo_execution::application::file_dag::frontiers::types::PublicationSlot;
use ryo_execution::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution::application::file_dag::program::file_session::types::FileInput;
use ryo_execution::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution::application::file_dag::program::prepared_execution::interface::PreparedExecution;
use ryo_execution::application::file_dag::program::prepared_execution::interface::PreparedReporting;
use ryo_execution::application::rule_compilation::service::ExecutionCompilation;
use ryo_execution::application::rule_compilation::service::PreparedQueries;
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason;
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityRefusal;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
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
/// The public traits of `interface.rs` that are no entry: each states why none of its methods admits or mutates a root.
const NON_ENTRY: [(&str, &str); 4] = [
    (
        "PreparedExecution",
        "read-only metadata of a prepared program: a rule outside it answers an error and nothing is held, admitted or changed",
    ),
    (
        "ExecutionCompilation",
        "compiles the rules a request declares into plans and identities before any root exists",
    ),
    (
        "PreparedQueries",
        "reads the plans and bindings a program was prepared from; it takes no rule, scope or intent",
    ),
    (
        "PreparedReporting",
        "reads the run metadata and the replay closure of a finished preparation; it takes no root",
    ),
];
#[test]
pub(super) fn every_public_trait_is_an_entry_with_variants_or_is_named_with_its_reason() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let interface =
        std::fs::read_to_string(root.join("src/inbound/use_cases/execution/interface.rs")).unwrap();
    let declared: Vec<&str> = interface
        .lines()
        .filter_map(|line| line.strip_prefix("pub trait "))
        .map(|rest| {
            rest.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .next()
                .unwrap_or_default()
        })
        .collect();
    assert!(!declared.is_empty(), "interface.rs declares public traits");
    for name in &declared {
        let entry = TRAITS.iter().any(|(each, _)| each == name);
        let reason = NON_ENTRY
            .iter()
            .find(|(each, _)| each == name)
            .map(|(_, reason)| *reason);
        assert!(
            entry != reason.is_some(),
            "{name} is in exactly one of TRAITS and NON_ENTRY: a new public trait is classified here before it passes"
        );
        if let Some(reason) = reason {
            assert!(!reason.is_empty(), "{name} states why it is no entry");
        }
    }
    for (name, _) in TRAITS.iter().chain(NON_ENTRY.iter()) {
        assert!(
            declared.contains(name),
            "{name} is listed but interface.rs declares no such public trait"
        );
    }
}
/// Every `.rs` file under `dir`, as paths relative to `base`.
fn sources_under(base: &Path, dir: &Path, found: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources_under(base, &path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(
                path.strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
}
/// A `PreparedFrontiers` value is made only by the `frontiers` module, where `FrontierCache::prepared` is the one route to it: a
/// literal in any other production file would make a handover that never read its sources once.
#[test]
pub(super) fn prepared_frontiers_are_built_only_by_the_frontiers_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources_under(root, &root.join("src"), &mut files);
    let mut builders = Vec::new();
    for file in files {
        let test_tree = file.split('/').any(|part| {
            part == "test" || part == "tests" || part == "fixtures" || part.ends_with("_test.rs") || part == "test.rs"
        });
        if test_tree {
            continue;
        }
        let text = std::fs::read_to_string(root.join(&file)).unwrap();
        let literals = text
            .lines()
            .map(str::trim_start)
            .filter(|line| line.contains("PreparedFrontiers {"))
            .filter(|line| !line.starts_with("pub struct ") && !line.starts_with("impl "))
            .count();
        if literals > 0 {
            builders.push((file, literals));
        }
    }
    assert!(!builders.is_empty(), "the frontiers module builds them");
    for (file, literals) in builders {
        assert!(
            file.starts_with("src/application/file_dag/frontiers/"),
            "{file} builds {literals} PreparedFrontiers outside the frontiers module"
        );
    }
}
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
        let methods = body.lines().filter(|line| line.starts_with("    fn ")).count();
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
