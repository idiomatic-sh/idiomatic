//! Submission routes come from the source inventory: a new entry or demand variant without a
//! control here fails this census instead of silently escaping it.
use crate::boundary_fixture::{corpus, engine, prepared_rules};
use crate::file_intents::named_request;
use crate::project_phase::negative::fixtures::{Entry, Root, submit};
use ryo_execution::inbound::use_cases::execution::{
    interface::{ExecutionRequest, ExecutionUseCase, FileExecution},
    types::*,
};
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::FileId;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const PROTOCOL: &str = "src/application/file_dag/program/protocol/types.rs";
const ADMISSION: &str = "src/application/file_dag/program/admission.rs";
/// Public traits whose implementations accept a set of declared roots or intents.
const SUBMISSION_TRAITS: [&str; 2] = ["ExecutionRequest", "ProjectRootExecution"];

fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}
fn identifier(text: &str) -> &str {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .next()
        .unwrap_or_default()
}
/// Every `impl <submission trait> for <Type>` in the production inbound layer.
fn entries(root: &Path) -> BTreeSet<String> {
    let mut files = Vec::new();
    rust_files(&root.join("src/inbound"), &mut files);
    let mut found = BTreeSet::new();
    for file in files {
        for line in fs::read_to_string(file).unwrap().lines() {
            let Some(mut rest) = line.strip_prefix("impl") else {
                continue;
            };
            if rest.starts_with('<') {
                let mut depth = 0;
                let end = rest
                    .char_indices()
                    .find(|(_, c)| {
                        depth += match c {
                            '<' => 1,
                            '>' => -1,
                            _ => 0,
                        };
                        depth == 0
                    })
                    .map_or(0, |(at, _)| at + 1);
                rest = &rest[end..];
            }
            let Some((head, tail)) = rest.trim_start().split_once(" for ") else {
                continue;
            };
            let head = identifier(head.trim());
            if SUBMISSION_TRAITS.contains(&head) {
                found.insert(format!("{head} for {}", identifier(tail.trim())));
            }
        }
    }
    found
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
}

#[test]
fn every_submission_route_refuses_an_incomplete_or_extra_root_set() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let routes: Vec<(&str, Box<dyn Fn()>)> = vec![
        ("ExecutionRequest for RequestExecution", Box::new(file_route)),
        (
            "ProjectRootExecution for ProjectPhase",
            Box::new(|| project_route(Entry::Phase)),
        ),
        (
            "ProjectRootExecution for ObservationStage",
            Box::new(|| project_route(Entry::Stage)),
        ),
    ];
    assert_eq!(
        entries(root),
        routes
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .collect::<BTreeSet<_>>(),
        "a submission entry gained or lost its implementation: add or remove its control"
    );
    assert_inventory(root, PROTOCOL, "DiagnosticDelivery", covered(&DELIVERIES));
    assert_inventory(root, PROTOCOL, "ObservationDemand", covered(&DEMANDS));
    assert_inventory(root, PROTOCOL, "CaptureIntent", covered(&CAPTURES));
    assert_inventory(root, PROTOCOL, "ApplicabilityRefusal", covered(&REFUSALS));
    assert_inventory(root, ADMISSION, "ApplicabilityReason", covered(&REASONS));
    assert_inventory(root, PROTOCOL, "FileRuleIntent", covered(&every_intent()));
    for (_, route) in routes {
        route();
    }
}
