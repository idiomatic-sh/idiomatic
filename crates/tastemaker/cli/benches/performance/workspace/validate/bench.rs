//! `validate`'s workload over the bench workspace.

use tastemaker_cli::resources::workspace::validate::command::{ValidateArguments, run};

/// The `validate` workload: the command run once per round through its own composition, its answer checked.
pub(crate) fn workload() -> crate::measure::Measured {
    let directory = crate::fixtures::workspace();
    let edge = crate::fixtures::edge(&directory);
    crate::measure::rounds(
        "workspace/validate",
        || crate::fixtures::arguments::<ValidateArguments>(&["validate"]),
        |arguments| {
            let outcome = run(arguments, &edge);
            assert_eq!(
                outcome,
                tastemaker_cli::outcome::Outcome::Success,
                "`validate` ends as it should"
            );
        },
    )
}
