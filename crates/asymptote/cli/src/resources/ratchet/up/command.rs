//! `up`: parses, checks, composes, calls the service, presents.

use crate::outcome::Outcome;

/// The arguments of `up`.
#[derive(Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling `schemas.rs` and that file's tests read them; `pub` would publish them through the library's public `UpArguments`, and private fields would hide them from those tests"
)]
#[command(
    after_help = "Examples:\n  asymptote up --reason \"new rule; burn-down tracked in the issue tracker\"\n  asymptote up --rule idiomatic.arch/file-length --at crates/core/src/lint.rs:1 --reason \"split tracked in the issue tracker\"\n  asymptote up --path 'crates/core/**' --reason \"the hex move, ratcheted down after\"\n  asymptote up --tool ryo-bench --reason \"seeded\"\n\nPerf: with --tool naming a perf tool (or no selector at all), the last check's medians that have no record, or whose `lower_value` is over their best, are written as perf records, best = the median, in the same operation as the findings: `--rule`, `--path` and `--at` name findings, and leave perf alone. A record raised keeps its earlier reasons and appends the --reason."
)]
pub struct UpArguments {
    /// Why these findings are accepted: it lands on every record written.
    #[arg(long)]
    pub(super) reason: String,
    /// Record only findings of this rule; repeat it for more.
    #[arg(long, value_name = "RULE")]
    pub(super) rule: Vec<String>,
    /// Record only findings in files this glob matches, relative to the product; repeat it for more.
    #[arg(long, value_name = "GLOB")]
    pub(super) path: Vec<String>,
    /// Record only findings, and perf medians, of this tool; repeat it for more.
    #[arg(long, value_name = "TOOL")]
    pub(super) tool: Vec<String>,
    /// Record only the new finding at PATH:LINE, as `check` prints it; repeat it for more.
    #[arg(long, value_name = "PATH:LINE")]
    pub(super) at: Vec<String>,
}

/// Checks the arguments, composes the products, calls each once and shapes their answer, with the outcome the command
/// ends as: an error when a product could not answer.
pub(super) fn answer(
    arguments: UpArguments,
    edge: &crate::edge::Edge,
) -> Result<(impl crate::output::Encoded + use<>, Outcome), Outcome> {
    let request = super::schemas::request(
        arguments.reason,
        arguments.rule,
        arguments.path,
        arguments.tool,
        arguments.at,
        edge,
    )
    .map_err(|error| crate::output::fail(&error, Outcome::Failure))?;
    let workspace = super::composition::build(
        &request.start,
        &super::connections::connect(&edge.build, &edge.store),
    )
    .map_err(|error| {
        crate::output::fail(
            &asymptote_core::inbound::use_cases::up::errors::UpError::from(error),
            Outcome::Error,
        )
    })?;
    // At a workspace root it reaches what `check` reaches there: the root's own tools, then each member.
    let answered = workspace
        .products
        .iter()
        .map(|product| {
            (
                product.path.0.to_string(),
                product
                    .service
                    .up(asymptote_core::inbound::use_cases::up::types::UpRequest {
                        start: product.root.clone(),
                        ..request.clone()
                    }),
            )
        })
        .collect();
    super::schemas::answered(workspace.layout, answered)
}

/// Runs `up`: records the last check's new findings the selectors choose, in one lock operation.
pub fn run(arguments: UpArguments, edge: &crate::edge::Edge) -> Outcome {
    match answer(arguments, edge) {
        // A dry run's answer is its plan, which goes where the `--dry-run` switch says.
        Ok(answered) => {
            crate::output::dry_run::present(edge.writing, edge.format, &answered.0, answered.1)
        }
        Err(outcome) => outcome,
    }
}
