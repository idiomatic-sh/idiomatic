//! Every method that accepts a declared root, scope or intent is an entry. The witness impls
//! below must define every method of each entry trait, so a new method fails to compile until it
//! names a `Method`; the exhaustive `control` match then fails to compile until that method has
//! a control here. Each control submits an incomplete, extra or undeclared root and expects refusal.
use crate::boundary_fixture::{corpus, engine, prepared_rules};
use crate::file_intents::named_request;
use crate::project_phase::negative::fixtures::{
    Entry, Held, Root, foreign_phase_outcome, prepare_for, submit, with_held,
};
use ryo_execution::inbound::use_cases::execution::{
    interface::{
        ExecutionRequest, ExecutionUseCase, FileExecution, InvocationStateCounts,
        ObservationExecution, ProjectRootExecution, ProjectStage, ReadinessCounts,
    },
    types::*,
};
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::{
    files::types::CorpusInput, observations::types::ObservationBindingId, runs::types::Skipped,
    stored_facts::types::ArenaBytes,
};
use std::{collections::BTreeSet, fs, path::Path};

const PROTOCOL: &str = "src/application/file_dag/program/protocol/types.rs";
const ADMISSION: &str = "src/application/file_dag/program/admission.rs";
fn identifier(text: &str) -> &str {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .next()
        .unwrap_or_default()
}
/// The variants a declaration lists, read from its source text.
fn variants(source: &str, name: &str) -> BTreeSet<String> {
    let start = source
        .find(&format!("enum {name} {{"))
        .unwrap_or_else(|| panic!("{name} is declared in its inventory source"));
    let mut depth = 0;
    let mut found = BTreeSet::new();
    for (at, line) in source[start..].lines().enumerate() {
        let trimmed = line.trim();
        if depth == 1
            && !trimmed.is_empty()
            && !trimmed.starts_with("//")
            && !trimmed.starts_with("#[")
            && !trimmed.starts_with(['}', ')'])
        {
            found.insert(identifier(trimmed).to_owned());
        }
        for c in line.chars() {
            depth += match c {
                '{' | '(' => 1,
                '}' | ')' => -1,
                _ => 0,
            };
        }
        if at > 0 && depth == 0 {
            break;
        }
    }
    found
}
fn covered<T: std::fmt::Debug>(values: &[T]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| identifier(&format!("{value:?}")).to_owned())
        .collect()
}
fn assert_inventory(root: &Path, file: &str, name: &str, covered: BTreeSet<String>) {
    let source = fs::read_to_string(root.join(file)).unwrap();
    assert_eq!(
        variants(&source, name),
        covered,
        "{name} gained or lost a variant: add or remove its submission control"
    );
}

const DELIVERIES: [DiagnosticDelivery; 3] = [
    DiagnosticDelivery::Disabled,
    DiagnosticDelivery::Collected,
    DiagnosticDelivery::Forwarded,
];
const DEMANDS: [ObservationDemand; 2] =
    [ObservationDemand::Required, ObservationDemand::NotRequired];
const CAPTURES: [CaptureIntent; 3] = [
    CaptureIntent::Disabled,
    CaptureIntent::Observations,
    CaptureIntent::DiagnosticAndObservations,
];
const REASONS: [ApplicabilityReason; 2] = [
    ApplicabilityReason::SyntaxRejected,
    ApplicabilityReason::DiagnosticNotApplicable,
];
const REFUSALS: [ApplicabilityRefusal; 2] = [
    ApplicabilityRefusal::SyntaxUndetermined,
    ApplicabilityRefusal::MetadataUnavailable,
];
fn every_intent() -> Vec<FileRuleIntent> {
    let mut all = vec![FileRuleIntent::Undemanded];
    all.extend(REASONS.map(FileRuleIntent::Skipped));
    all.extend(REFUSALS.map(FileRuleIntent::Refused));
    for diagnostic in DELIVERIES {
        for observations in DEMANDS {
            for capture in CAPTURES {
                all.push(FileRuleIntent::Run {
                    diagnostic,
                    observations,
                    capture,
                });
            }
        }
    }
    all
}

/// File route: one intent per prepared rule, for every intent kind, over independent and named programs.
fn file_route() {
    let engine = engine();
    let programs = [
        prepared_rules(&engine, 2),
        engine.prepare(named_request()).expect("named request"),
    ];
    let corpus = corpus();
    let elements = Elements::default();
    let read = |_| Err(OperatingError::NotRead);
    let open = |request: &RequestExecution, intents: &[FileRuleIntent]| {
        let mut symbols = Symbols::default();
        request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: facts::ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents,
                    source: b"abc",
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .map(|file| {
                let _ = file.finish();
            })
    };
    for program in &programs {
        for intent in every_intent() {
            let request = engine.begin(program, &corpus);
            for length in [0, 1, 3] {
                assert!(
                    matches!(
                        open(&request, &vec![intent; length]),
                        Err(OperatingError::ShapeMismatch)
                    ),
                    "{intent:?} x {length} intents must be refused as incomplete or extra"
                );
            }
            request.finish();
            let request = engine.begin(program, &corpus);
            assert!(
                !matches!(
                    open(&request, &[intent, intent]),
                    Err(OperatingError::ShapeMismatch)
                ),
                "{intent:?} x 2 intents is the complete declared set"
            );
            request.finish();
        }
    }
}
/// Project routes: ordinary and named roots; collected delivery and the host guard callback.
fn project_route(entry: Entry) {
    let engine = engine();
    let root = |rule| Root {
        declared: rule,
        named: rule,
    };
    let extra = Root {
        declared: 0,
        named: 3,
    };
    let complete = submit(&engine, entry, 3, &[root(0), root(1), root(2)], None);
    assert_eq!(complete.outcomes.len(), 3);
    let omitted = submit(&engine, entry, 3, &[root(0), root(2)], None);
    assert_eq!(omitted.outcomes.len(), 2);
    assert!(!omitted.guarded.contains(&RuleIndex(1)));
    let collected = submit(&engine, entry, 3, &[root(0), extra, root(2)], None);
    assert_eq!(collected.outcomes.len(), 3);
    assert!(matches!(collected.outcomes[1], Err(OperatingError::NotRun)));
    let callback = submit(
        &engine,
        entry,
        3,
        &[root(0), extra, root(2)],
        Some(RuleIndex(3)),
    );
    assert!(matches!(callback.refused, Some(OperatingError::WrongBinding)));
    assert_eq!(callback.outcomes.len(), 1);
    // In range, but the session declared rule 0: refused, never run undeclared.
    let undeclared = Root {
        declared: 0,
        named: 1,
    };
    let mismatch = submit(&engine, entry, 3, &[root(0), undeclared, root(2)], None);
    assert_eq!(mismatch.outcomes.len(), 3);
    assert!(mismatch.outcomes[0].is_ok() && mismatch.outcomes[2].is_ok());
    assert!(matches!(mismatch.outcomes[1], Err(OperatingError::NotRead)));
    // A declared root submitted twice keeps one outcome per submission.
    let twice = submit(&engine, entry, 3, &[root(0), root(0)], None);
    assert_eq!(twice.outcomes.len(), 2);
    assert!(twice.outcomes.iter().all(Result::is_ok));
}

/// One variant per method of every entry trait. `witness` below maps each trait method here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Method {
    RequestObservations,
    RequestPrepareFrontiers,
    RequestFinish,
    RequestOpenFile,
    RequestProjectPhase,
    RootsEvaluate,
    RootsReadinessCounts,
    StageStateCounts,
    StageProjectScope,
    StageFinish,
    ObservedReserve,
    ObservedBindings,
    ObservedBindingReadiness,
    ObservedArenas,
    ObservedProjectScope,
    ObservedFinish,
    ObservedFinishStage,
}
const ALL: [Method; 17] = [
    Method::RequestObservations,
    Method::RequestPrepareFrontiers,
    Method::RequestFinish,
    Method::RequestOpenFile,
    Method::RequestProjectPhase,
    Method::RootsEvaluate,
    Method::RootsReadinessCounts,
    Method::StageStateCounts,
    Method::StageProjectScope,
    Method::StageFinish,
    Method::ObservedReserve,
    Method::ObservedBindings,
    Method::ObservedBindingReadiness,
    Method::ObservedArenas,
    Method::ObservedProjectScope,
    Method::ObservedFinish,
    Method::ObservedFinishStage,
];
/// Its methods never run: implementing every method of each entry trait is what the compiler checks.
struct Witness;
fn implements_every_entry_trait<
    W: ExecutionRequest + ProjectRootExecution + ProjectStage + ObservationExecution,
>(
    _: &W,
) {
}
fn witness(method: Method) -> ! {
    unreachable!("{method:?} is a compile-time witness only")
}
impl ExecutionRequest for Witness {
    fn observations<'a>(
        &'a self,
        _: &'a mut Vec<ArenaBytes>,
    ) -> Result<ObservationStage<'a>, OperatingError> {
        witness(Method::RequestObservations)
    }
    fn prepare_frontiers(
        &self,
        _: FrontierRequest<'_>,
        _: &mut Symbols,
    ) -> Result<PreparedFrontiers, PreparationError> {
        witness(Method::RequestPrepareFrontiers)
    }
    fn finish(self) {
        witness(Method::RequestFinish)
    }
    fn open_file<'a, 'facts: 'a>(
        &'a self,
        _: FileInput<'a, 'facts>,
        _: &mut Symbols,
    ) -> Result<FileSession<'a, 'facts>, OperatingError> {
        witness(Method::RequestOpenFile)
    }
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError> {
        witness(Method::RequestProjectPhase)
    }
}
impl ProjectRootExecution for Witness {
    fn evaluate<E>(
        &self,
        _: Vec<ProjectTask<'_>>,
        _: &mut Symbols,
        _: impl FnMut(RuleIndex) -> Result<(), E>,
    ) -> ProjectBatch<Result<ProjectRuleOutcome, OperatingError>, E> {
        witness(Method::RootsEvaluate)
    }
    fn readiness_counts(&self) -> ReadinessCounts {
        witness(Method::RootsReadinessCounts)
    }
}
impl ProjectStage for Witness {
    fn state_counts(&self) -> InvocationStateCounts {
        witness(Method::StageStateCounts)
    }
    fn project_scope(
        &self,
        _: RuleIndex,
        _: &[CorpusInput],
        _: &[Covered],
        _: &[Skipped],
    ) -> Result<ProjectScope, OperatingError> {
        witness(Method::StageProjectScope)
    }
    fn finish(self) -> Result<(), OperatingError> {
        witness(Method::StageFinish)
    }
}
impl ObservationExecution for Witness {
    fn reserve(&mut self, _: RuleIndex) -> Result<PublicationSlot, OperatingError> {
        witness(Method::ObservedReserve)
    }
    fn bindings(&self, _: RuleIndex) -> Result<Vec<BoundInput>, OperatingError> {
        witness(Method::ObservedBindings)
    }
    fn binding_readiness(
        &self,
        _: RuleIndex,
    ) -> Result<std::collections::BTreeMap<ObservationBindingId, bool>, OperatingError> {
        witness(Method::ObservedBindingReadiness)
    }
    fn arenas(&self) -> &[ArenaBytes] {
        witness(Method::ObservedArenas)
    }
    fn project_scope(
        &self,
        _: RuleIndex,
        _: &[CorpusInput],
        _: &[Covered],
        _: &[Skipped],
    ) -> Result<ProjectScope, OperatingError> {
        witness(Method::ObservedProjectScope)
    }
    fn finish(&mut self, _: RuleIndex) -> Result<(), OperatingError> {
        witness(Method::ObservedFinish)
    }
    fn finish_stage(self) -> Result<(), OperatingError> {
        witness(Method::ObservedFinishStage)
    }
}

/// What a method does with a root set: refuse an incomplete, extra or undeclared one, or
/// accept none at all (its inputs carry no rule, scope or intent to declare).
enum Control {
    Refuses(fn()),
    AcceptsNoRoot(&'static str),
}
fn control(method: Method) -> Control {
    match method {
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
        Method::RequestFinish => Control::AcceptsNoRoot("drops the request"),
        Method::RootsReadinessCounts => Control::AcceptsNoRoot("reads counters"),
        Method::StageStateCounts => Control::AcceptsNoRoot("reads counters"),
        Method::StageFinish => Control::AcceptsNoRoot("releases the phase"),
        Method::ObservedArenas => Control::AcceptsNoRoot("reads published arenas"),
        Method::ObservedFinishStage => Control::AcceptsNoRoot("releases the stage"),
    }
}
fn evaluate_routes() {
    project_route(Entry::Phase);
    project_route(Entry::Stage);
}
fn observations_need_a_named_program() {
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
fn frontiers_of_an_undeclared_rule() {
    with_held(&engine(), Entry::Phase, 3, |request, held, corpus, covered| {
        let scope = held.try_scope(RuleIndex(0), corpus, covered).unwrap();
        assert!(prepare_for(request, &scope, RuleIndex(0)).is_ok());
        assert!(matches!(
            prepare_for(request, &scope, RuleIndex(3)),
            Err(PreparationError::Metadata(OperatingError::NotRead))
        ));
    });
}
fn a_scope_belongs_to_its_own_phase() {
    assert!(matches!(
        foreign_phase_outcome(&engine()),
        Err(OperatingError::NotRead)
    ));
}
fn scope_of_an_undeclared_rule(entry: Entry) {
    with_held(&engine(), entry, 3, |_, held, corpus, covered| {
        assert!(held.try_scope(RuleIndex(2), corpus, covered).is_ok());
        assert!(matches!(
            held.try_scope(RuleIndex(3), corpus, covered),
            Err(OperatingError::NotRun)
        ));
    });
}
fn observed_undeclared_rule() {
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

#[test]
fn every_entry_method_refuses_an_incomplete_extra_or_undeclared_root() {
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
    for method in ALL {
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
    assert_eq!(refused + exempt, ALL.len(), "every entry method was classified");
}
