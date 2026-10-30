//! `create`'s workload over the bench workspace.

use tastemaker_cli::resources::workspace::create::command::{CreateArguments, run};

/// The `create` workload: the command run once per round through its own composition, its answer checked.
pub(crate) fn workload() -> crate::measure::Measured {
    let directory = crate::fixtures::workspace();
    let edge = crate::fixtures::dry(crate::fixtures::edge(&directory));
    crate::measure::rounds(
        "workspace/create",
        || {
            crate::fixtures::arguments::<CreateArguments>(&[
                "create",
                "rule/no-tsv",
                "--set",
                "extension=rs",
            ])
        },
        |arguments| {
            let outcome = run(arguments, &edge);
            assert_eq!(
                outcome,
                tastemaker_cli::outcome::Outcome::Success,
                "`create` ends as it should"
            );
        },
    )
}
