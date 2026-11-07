//! `scaffold`'s workload over the bench workspace.

use tastemaker_cli::resources::workspace::scaffold::command::{ScaffoldArguments, run};

/// The `scaffold` workload: the command run once per round through its own composition, its answer checked.
pub(crate) fn workload() -> crate::measure::Measured {
    let directory = crate::fixtures::workspace();
    let edge = crate::fixtures::dry(crate::fixtures::edge(&directory));
    crate::measure::rounds(
        "workspace/scaffold",
        || crate::fixtures::arguments::<ScaffoldArguments>(&["scaffold", "part", "--at", "src/c"]),
        |arguments| {
            let outcome = run(arguments, &edge);
            assert_eq!(
                outcome,
                tastemaker_cli::outcome::Outcome::Success,
                "`scaffold` ends as it should"
            );
        },
    )
}
