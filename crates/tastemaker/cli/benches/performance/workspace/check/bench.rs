//! `check`'s workloads: over the bench workspace, and warm over a workspace of fifty instances, through the command line's
//! own composition, its lock written and its plan kept by the `check --fix` before.

use tastemaker_cli::resources::workspace::check::command::{CheckArguments, run};

/// The `check` workload: the command run once per round through its own composition, its answer checked.
pub(crate) fn workload() -> crate::measure::Measured {
    let directory = crate::fixtures::workspace();
    let edge = crate::fixtures::edge(&directory);
    crate::measure::rounds(
        "workspace/check",
        || crate::fixtures::arguments::<CheckArguments>(&["check"]),
        |arguments| {
            let outcome = run(arguments, &edge);
            assert_eq!(
                outcome,
                tastemaker_cli::outcome::Outcome::Failure,
                "`check` ends as it should"
            );
        },
    )
}

/// A workspace of fifty `src/<domain>` instances, its lock written and its plan kept.
#[expect(
    clippy::expect_used,
    reason = "a benchmark's setup fails by panicking: a scene that cannot be written or a line that cannot be parsed is a broken benchmark, and the message names which"
)]
fn instance_workspace() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let write = |path: &str, text: &str| {
        let place = directory.path().join(path);
        std::fs::create_dir_all(place.parent().expect("a parent")).expect("its directory");
        std::fs::write(place, text).expect("the file");
    };
    std::fs::create_dir_all(directory.path().join(".git")).expect("a .git directory");
    write(
        "tastemaker.toml",
        "workspace = { name = \"w\", dependencies = [{ package = \"p\", path = \"p\" }] }\nclosed = false\nspecs = [{ spec = \"p/part\", include = [\"src/{domain}\"] }]\n",
    );
    write("p/package.toml", "name = \"p\"\n");
    write("p/specs/part/0.1.0/spec.toml", crate::fixtures::SPEC);
    write(
        "p/specs/part/0.1.0/templates/errors/template.toml",
        crate::fixtures::TEMPLATE,
    );
    for index in 0_u32..50_u32 {
        write(&format!("src/d{index}/errors.rs"), "pub enum Error {}\n");
    }
    directory
}

/// The warm `check` workload over a workspace of fifty instances, taken after a first run wrote its lock and kept its plan.
#[expect(
    clippy::expect_used,
    reason = "a benchmark's setup fails by panicking: a scene that cannot be written or a line that cannot be parsed is a broken benchmark, and the message names which"
)]
pub(crate) fn instances() -> crate::measure::Measured {
    let directory = instance_workspace();
    let edge = tastemaker_cli::resources::edge_at(
        tastemaker_core::domain::files::types::MachinePath(
            directory.path().display().to_string().into(),
        ),
        tastemaker_core::domain::settings::types::Build("bench".into()),
        tastemaker_cli::output::Format::Quiet,
        tastemaker_core::domain::writing::types::Writing::Write,
    );
    let line = |words: &[&str]| {
        let command = <CheckArguments as clap::Args>::augment_args(clap::Command::new("check"));
        <CheckArguments as clap::FromArgMatches>::from_arg_matches(
            &command
                .try_get_matches_from(words)
                .expect("the line parses"),
        )
        .expect("the arguments")
    };
    let validation = <tastemaker_cli::resources::workspace::validate::command::ValidateArguments as clap::Args>::augment_args(clap::Command::new("validate"));
    let arguments = <tastemaker_cli::resources::workspace::validate::command::ValidateArguments as clap::FromArgMatches>::from_arg_matches(
        &validation.try_get_matches_from(["validate"]).expect("the line parses"),
    ).expect("the arguments");
    assert_eq!(
        tastemaker_cli::resources::workspace::validate::command::run(arguments, &edge),
        tastemaker_cli::outcome::Outcome::Success,
        "the fifty-instance workspace validates"
    );
    assert_eq!(
        run(line(&["check", "--fix"]), &edge),
        tastemaker_cli::outcome::Outcome::Success,
        "the fifty-instance workspace places its lock"
    );
    crate::measure::rounds(
        "workspace/check-fifty-instances",
        || (),
        |()| {
            assert_eq!(
                run(line(&["check"]), &edge),
                tastemaker_cli::outcome::Outcome::Success,
                "`check` over fifty instances ends as it should"
            );
        },
    )
}
