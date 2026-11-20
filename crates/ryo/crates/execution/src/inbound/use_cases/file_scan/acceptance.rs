//! The file scan feature enters the real public use case, over the engine port's fake and the fixture language.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_execution::inbound::use_cases::file_scan::fixtures::{Edge, composed, file, rule};
use ryo_execution::inbound::use_cases::file_scan::interface::FileScanUseCase;
use ryo_execution::inbound::use_cases::file_scan::types::{ScanLoadRequest, Scanned};
use ryo_values::domain::stored_facts::types::Digest;

#[given(expr = "a file scan request {string}")]
fn request(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "the file {string} is scanned")]
fn scanned(world: &mut Behavior, path: String) {
    let (scan, _fake) = composed(match world.input.as_str() {
        "old engine" => Edge::Old,
        "no engine" => Edge::NoEngine,
        _ => Edge::New,
    });
    let plan = rule(&["id", "file", "span"]);
    let loaded = scan.load(ScanLoadRequest {
        plans: &[&plan],
        given: Digest([1; 16]),
        stored: &[],
    });
    world.answer = match loaded {
        Ok(session) => {
            let (mut worker, mut rows) = (scan.worker(&session), Vec::new());
            let answered = scan.scan(&session, &mut worker, file(0, &path, b"one two"), &mut rows);
            match answered {
                Scanned::Whole(_) => format!("{} rows", rows.len()),
                Scanned::Faulted(_, fault) | Scanned::NotWalked(fault) => {
                    format!("{} rows; {}", rows.len(), fault.sentence.0)
                }
            }
        }
        Err(_) => "refused".to_owned(),
    };
}

#[then(expr = "the file scan answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
