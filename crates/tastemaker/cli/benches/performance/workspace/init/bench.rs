//! Initialization planning performs no package discovery or downloads.
use tastemaker_cli::resources::workspace::init::command::{InitArguments, run};
/// The `init` workload: the command run once per round as a dry run in an empty project, its answer checked.
#[expect(
    clippy::expect_used,
    reason = "a benchmark must fail when its empty project cannot be created; continuing would measure a different workload"
)]
pub(crate) fn workload() -> crate::measure::Measured {
    let directory = tempfile::tempdir().expect("empty project");
    // A project is a git repository: `init` refuses one outside any.
    std::fs::create_dir_all(directory.path().join(".git")).expect("the repository's marker");
    let edge = crate::fixtures::dry(crate::fixtures::edge(&directory));
    crate::measure::rounds(
        "workspace/init",
        || crate::fixtures::arguments::<InitArguments>(&["init"]),
        |arguments| {
            assert_eq!(
                run(arguments, &edge),
                tastemaker_cli::outcome::Outcome::Success,
                "`init` ends as it should"
            );
        },
    )
}
