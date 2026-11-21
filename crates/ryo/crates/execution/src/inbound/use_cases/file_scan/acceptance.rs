//! The file scan feature enters the real public use case, over the engine port's fake and the fixture language.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_execution::inbound::use_cases::file_scan::fixtures::{Edge, composed, file, rule};
use ryo_execution::inbound::use_cases::file_scan::interface::FileScanUseCase;
use ryo_execution::inbound::use_cases::file_scan::types::{ScanLoadRequest, Scanned};
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::Digest;

#[given(expr = "a file scan request {string}")]
fn request(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "the file {string} is scanned")]
fn scanned(world: &mut Behavior, path: String) {
    world.answer = answered(&world.input, &[&path]);
}

#[when(expr = "the files {string} and {string} are scanned")]
fn scanned_both(world: &mut Behavior, first: String, second: String) {
    world.answer = answered(&world.input, &[&first, &second]);
}

/// What the scan of `paths`, numbered from nought in one worker, says at the edge the input names: a line a file, then
/// each warning of what the engine noticed, as the use case words it for the rule `rule`.
fn answered(input: &str, paths: &[&str]) -> String {
    let (scan, _fake) = composed(match input {
        "old engine" => Edge::Old,
        "no engine" => Edge::NoEngine,
        "new engine, a rule overruns in b.walk" => Edge::Overrun,
        "new engine, set to fail on an overrun in b.walk" => Edge::Gate,
        _ => Edge::New,
    });
    let plan = rule(&["id", "file", "span"]);
    let loaded = scan.load(ScanLoadRequest {
        plans: &[&plan],
        bindings: &[],
        statistics: ByteSlice(&[]),
        given: Digest([1; 16]),
        stored: &[],
    });
    let Ok(session) = loaded else {
        return "refused".to_owned();
    };
    let (mut worker, mut lines, mut warnings) = (scan.worker(&session), Vec::new(), Vec::new());
    let id = RuleId("rule".into());
    for (number, path) in (0..).zip(paths) {
        let (mut rows, mut noticed) = (Vec::new(), Vec::new());
        let scanned = scan.scan(
            &session,
            &mut worker,
            file(number, path, b"one two"),
            &mut rows,
            &mut noticed,
        );
        let said = match scanned {
            Scanned::Whole(_) => format!("{} rows", rows.len()),
            Scanned::Overran(_) => format!("{} rows; the run fails", rows.len()),
            Scanned::Faulted(_, fault) | Scanned::NotWalked(fault) => {
                format!("{} rows; {}", rows.len(), fault.sentence.0)
            }
        };
        // One file scanned says its answer alone, as before; several say whose each answer is.
        lines.push(if paths.len() == 1 {
            said
        } else {
            format!("{path}: {said}")
        });
        let worded = noticed
            .iter()
            .map(|each| scan.worded(each, &id, SourcePath(path), None, None));
        warnings.extend(worded.map(|warning| warning.0.into_string()));
    }
    lines.extend(warnings);
    lines.join("\n")
}

#[then(expr = "the file scan says")]
fn says(world: &mut Behavior, #[step] step: &cucumber::gherkin::Step) {
    let expected = step.docstring.as_deref().unwrap_or_default();
    assert_eq!(world.answer.trim(), expected.trim());
}

#[then(expr = "the file scan answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
