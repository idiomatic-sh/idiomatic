//! `create`: parses, checks, composes, calls the service, presents.

use tastemaker_core::inbound::use_cases::create::interface::CreateUseCase as _;

use crate::outcome::Outcome;

/// The arguments of `create`.
#[derive(Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling `schemas.rs` and that file's tests read them; `pub` would publish them through the library's public arguments struct, and private fields would hide them from those tests"
)]
#[command(
    after_help = "Examples:\n  tm create rule/<name> --set extension=<ext>\n  tm create template/<name> --set extension=<ext>\n  tm create spec/<name> --package <directory> -n"
)]
pub struct CreateArguments {
    /// What to create: `rule/<name>`, `spec/<name>` or `template/<name>`.
    pub(super) definition: String,
    /// The package directory it is created in; the workspace's first package by path when left out.
    #[arg(long)]
    pub(super) package: Option<std::path::PathBuf>,
    /// A value the skeleton reads, `name=value`; repeat for more. A rule and a template read `extension`, the
    /// extension their scenario files end in, which Tastemaker never guesses and which must be one a linked language reads
    /// (`tm -V` names them): any other is refused, as its scenarios would find nothing. A name no skeleton reads is
    /// refused.
    #[arg(long = "set", value_name = "NAME=VALUE")]
    pub(super) set: Vec<String>,
}

/// Checks the arguments, composes the service, calls it once and shapes its answer.
pub(super) fn answer(
    arguments: CreateArguments,
    edge: &crate::edge::Edge,
) -> Result<impl crate::output::Encoded, Outcome> {
    let dry = edge.writing == tastemaker_core::domain::writing::types::Writing::DryRun;
    let request = super::schemas::request(
        arguments.definition,
        arguments.package,
        arguments.set,
        edge,
        &super::composition::languages(),
    )
    .map_err(|error| crate::output::fail(&error, Outcome::Error))?;
    super::composition::build(&edge.store)
        .create(request)
        .map(|report| super::schemas::answer(&report, dry, &edge.start))
        .map_err(|error| crate::output::fail(&error, Outcome::Error))
}

/// Runs `create`.
pub fn run(arguments: CreateArguments, edge: &crate::edge::Edge) -> Outcome {
    match answer(arguments, edge) {
        Ok(answer) => {
            crate::output::present_planned(edge.format, edge.writing, &answer, Outcome::Success)
        }
        Err(outcome) => outcome,
    }
}
