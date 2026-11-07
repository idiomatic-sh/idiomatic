//! `explain`'s workload over the bench workspace.

use tastemaker_cli::resources::workspace::explain::command::{ExplainArguments, run};

/// The `explain` workload: the command run once per round through its own composition, its answer checked.
pub(crate) fn workload() -> crate::measure::Measured {
    let directory = crate::fixtures::workspace();
    let edge = crate::fixtures::edge(&directory);
    crate::measure::rounds(
        "workspace/explain",
        || crate::fixtures::arguments::<ExplainArguments>(&["explain", "src/a/errors.rs"]),
        |arguments| {
            let outcome = run(arguments, &edge);
            assert_eq!(
                outcome,
                tastemaker_cli::outcome::Outcome::Success,
                "`explain` ends as it should"
            );
        },
    )
}
