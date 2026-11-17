//! What a rule entry's selectors can name over the `check` fixture's workspace, read by the manifest adapter and matched
//! by the matching adapter.

use crate::cases::{Case, CaseName, Expect, Word, run};
use tastemaker_values::domain::applications::service::known;
use crate::inbound::use_cases::check::fixtures::applications::{placement, workspace};

/// The fixture's tree.
const FILES: &[&str] = &[
    "tastemaker.toml",
    "src/a/errors.txt",
    "src/a/stray.txt",
    "src/b/notes.txt",
];

type Input = ();
/// What the row said, empty when it said what was expected.
type Value = String;
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("each_instance_and_each_placed_file_can_be_named"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(String::new()),
}];

fn subject((): Input) -> Value {
    let workspace = workspace(FILES);
    let placement = placement(&workspace);
    let known = known(&workspace, &placement);
    let said = format!(
        "{} | {}",
        known
            .instances
            .iter()
            .map(|instance| format!("{} {}", instance.directory.0, instance.spec.0))
            .collect::<Vec<_>>()
            .join(", "),
        known
            .placed
            .iter()
            .map(|placed| format!("{} {}", placed.path.0, placed.entry.0))
            .collect::<Vec<_>>()
            .join(", ")
    );
    if said == "src/a part, src/b part | src/a/errors.txt errors" {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
