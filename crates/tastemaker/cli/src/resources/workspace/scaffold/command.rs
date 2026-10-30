//! `scaffold`: parses, checks, composes, calls the service, presents.

use tastemaker_core::inbound::use_cases::scaffold::interface::ScaffoldUseCase as _;

use crate::outcome::Outcome;

/// The arguments of `scaffold`.
#[derive(Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling `schemas.rs` and that file's tests read them; `pub` would publish them through the library's public arguments struct, and private fields would hide them from those tests"
)]
#[command(
    after_help = "Examples:\n  tm scaffold <entry> --at <path>\n  tm scaffold <entry> --set <name>=<value> -n"
)]
pub struct ScaffoldArguments {
    /// The entry: its alias or its definition's name, or the name `tm explain` gives it (`package/name`,
    /// `package/spec/name`, `@version` after it), which names an entry of a spec instance at `--at`.
    pub(super) entry: String,
    /// The directory a spec entry's instance is written at, relative to the current directory: one its `include` selects.
    #[arg(long, default_value = ".")]
    pub(super) at: std::path::PathBuf,
    /// A capture or param the files read, `name=value`; repeat for more. A name no file reads is refused.
    #[arg(long = "set", value_name = "NAME=VALUE")]
    pub(super) set: Vec<String>,
}

/// Checks the arguments, composes the service, calls it once and shapes its answer.
pub(super) fn answer(
    arguments: ScaffoldArguments,
    edge: &crate::edge::Edge,
) -> Result<impl crate::output::Encoded, Outcome> {
    let dry = edge.writing == tastemaker_core::domain::writing::types::Writing::DryRun;
    let request = super::schemas::request(arguments.entry, arguments.at, arguments.set, edge)
        .map_err(|error| crate::output::fail(&error, Outcome::Failure))?;
    super::composition::build(&edge.store)
        .scaffold(request)
        .map(|report| super::schemas::answer(report, dry))
        .map_err(|error| crate::output::fail(&error, Outcome::Error))
}

/// Runs `scaffold`: writes the files the entry places that do not exist, never overwriting one.
pub fn run(arguments: ScaffoldArguments, edge: &crate::edge::Edge) -> Outcome {
    match answer(arguments, edge) {
        Ok(answer) => {
            crate::output::present_planned(edge.format, edge.writing, &answer, Outcome::Success)
        }
        Err(outcome) => outcome,
    }
}
