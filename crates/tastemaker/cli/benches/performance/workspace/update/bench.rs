//! `update`'s workload over the bench workspace.

use tastemaker_cli::resources::workspace::update::command::{UpdateArguments, run};

/// The `update` workload: the command run once per round through its own composition, its answer checked.
pub(crate) fn workload() -> crate::measure::Measured {
    let directory = crate::fixtures::workspace();
    let edge = crate::fixtures::dry(crate::fixtures::edge(&directory));
    crate::measure::rounds(
        "workspace/update",
        || crate::fixtures::arguments::<UpdateArguments>(&["update"]),
        |arguments| {
            let outcome = run(arguments, &edge);
            assert_eq!(
                outcome,
                tastemaker_cli::outcome::Outcome::Success,
                "`update` ends as it should"
            );
        },
    )
}
