//! `list`'s workload over the bench workspace.

use tastemaker_cli::resources::workspace::list::command::{ListArguments, run};

/// The `list` workload: the command run once per round through its own composition, its answer checked.
pub(crate) fn workload() -> crate::measure::Measured {
    let directory = crate::fixtures::workspace();
    let edge = crate::fixtures::edge(&directory);
    crate::measure::rounds(
        "workspace/list",
        || crate::fixtures::arguments::<ListArguments>(&["list"]),
        |arguments| {
            let outcome = run(arguments, &edge);
            assert_eq!(
                outcome,
                tastemaker_cli::outcome::Outcome::Success,
                "`list` ends as it should"
            );
        },
    )
}
