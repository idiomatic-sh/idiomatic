//! File and project submission routes over independent and named programs.
use super::inventory::every_intent;
use crate::boundary_fixture::{corpus, engine, prepared_rules};
use crate::file_intents::named_request;
use crate::project_phase::negative::fixtures::{Entry, Root, submit};
use ryo_execution::inbound::use_cases::execution::{
    interface::{
        ExecutionRequest, ExecutionUseCase, FileExecution, ObservationExecution,
        ProjectRootExecution, ProjectStage,
    },
    types::*,
};
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::FileId;

/// File route: one intent per prepared rule, for every intent kind, over independent and named programs.
pub(super) fn file_route() {
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
pub(super) fn project_route(entry: Entry) {
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
    assert!(matches!(
        callback.refused,
        Some(OperatingError::WrongBinding)
    ));
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
