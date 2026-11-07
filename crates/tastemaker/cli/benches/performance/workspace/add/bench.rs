//! `add`'s workload over the bench workspace.

use tastemaker_cli::resources::workspace::add::command::{AddArguments, run};

/// The `add` workload: the command run once per round through its own composition, its answer checked.
pub(crate) fn workload() -> crate::measure::Measured {
    let directory = crate::fixtures::workspace();
    let edge = crate::fixtures::dry(crate::fixtures::edge(&directory));
    crate::measure::rounds(
        "workspace/add",
        || crate::fixtures::arguments::<AddArguments>(&["add", "part", "--at", "src/c"]),
        |arguments| {
            let outcome = run(arguments, &edge);
            assert_eq!(
                outcome,
                tastemaker_cli::outcome::Outcome::Success,
                "`add` ends as it should"
            );
        },
    )
}
