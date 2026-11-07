//! `init`: parses, checks, composes, calls the service, presents.

use asymptote_core::inbound::use_cases::init::interface::InitUseCase as _;

use crate::outcome::Outcome;

/// The arguments of `init`.
#[derive(Clone, Copy, Debug, clap::Args)]
#[command(
    after_help = "Examples:\n  asymptote init\n  asymptote init --dry-run\n  asymptote init --root my-project\n\nRun `asymptote init` once in a git repository. Where there is no asymptote.toml it writes a minimal one and an empty lock, at the repository's top level, where the hook runs; an existing one is left as it is. It installs one script for each entry of the root asymptote.toml's `[hooks]` table (none when it has no table; the asymptote.toml it writes maps `pre-commit` to the lint group) in the repository's hooks directory, wherever git keeps it, each running `asymptote check --hook <name>`: the commit being made is checked by the hook. Commit messages are checked by the root's `commitizen` lint tool over the branch's commits, so the commit being made is checked at the next commit or in CI.\n\nRerunning it rewrites its own scripts and removes those it wrote for hooks no longer listed. It never touches a hook it did not write: it refuses, naming the file."
)]
pub struct InitArguments;

/// Composes the service, calls it once and shapes its answer.
pub(super) fn answer(
    _arguments: InitArguments,
    edge: &crate::edge::Edge,
) -> Result<impl crate::output::Encoded + use<>, Outcome> {
    super::composition::build(&edge.start)
        .init(super::schemas::request(edge))
        .map(|report| super::schemas::answer(&report))
        .map_err(|error| crate::output::fail(&error, super::schemas::outcome_of(&error)))
}

/// Runs `init`: writes the configuration and the hook scripts, or says what it would write under `--dry-run`.
pub fn run(arguments: InitArguments, edge: &crate::edge::Edge) -> Outcome {
    match answer(arguments, edge) {
        // A dry run's answer is its plan, which goes where the `--dry-run` switch says.
        Ok(answer) => {
            crate::output::dry_run::present(edge.writing, edge.format, &answer, Outcome::Success)
        }
        Err(outcome) => outcome,
    }
}
