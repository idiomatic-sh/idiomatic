//! `check`: parses, checks, composes, calls the service, presents.

use asymptote_core::inbound::use_cases::check::types::CheckRequest;

use crate::outcome::Outcome;

/// The arguments of `check`.
#[derive(Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling `schemas.rs` and that file's tests read them; `pub` would publish them through the library's public `CheckArguments`, where Tastemaker's `no-public-options` refuses a public `Option`, and private fields would hide them from those tests"
)]
#[command(
    after_help = "Examples:\n  asymptote check\n  asymptote check --since origin/main\n  asymptote check --fix --format sarif\n  asymptote check --groups lint,check\n  asymptote check --groups perf\n  asymptote check --hook pre-commit\n\nGroups: every tool in asymptote.toml has a `group`, a label of your choosing that asymptote gives no meaning; its `output` is read the same in any group. A plain `check` runs every tool, and `--groups a,b` only the tools labelled a or b. Tools outside the groups named are not run, not reported, and their lock records are not judged. A tool whose `run` names a git range (`origin/main..HEAD`) is kept from the cache only while the commits the range's ends resolve to are the same. An input that writes a capture, `{name}` for one path segment as in `languages/{lang}`, makes the tool one tool per entry it matches, `<id>/<value>`, each run, cached and reported on its own, with the value written into its `run`, `fix` and inputs: a new directory is a new tool. A `run` or `fix` that writes a capture no input does is refused.\n\nHooks: the root asymptote.toml's `[hooks]` table maps a git hook to the groups it runs (none when it has no `[hooks]` table, which `--hook` refuses), and `asymptote init` installs a script for each that runs `asymptote check --hook <name>`. Inside a member, `--hook` reads the root's table.\n\nPerf: a tool with `output = \"bmf\"` reports medians with an optional spread (`lower_value`, `upper_value`). A run whose `lower_value` is over its record's best fails; one with no record fails as unseeded (seed it with `asymptote up --reason <why> --tool <id>`); one whose `upper_value` is under the best rewrites the best to the run's `value` in the lock, and passes: this is the one place `check` writes the lock, and only a plain check writes it (`--fix` and `--since` never do), so commit it. A run that reports no spread is compared by its `value` alone.\n\nWorkspace: at a root whose asymptote.toml names `members` (globs of directories that each hold an asymptote.toml), `check` runs the root's tools, then each member's against its own lock, cache and report, and prefixes each line with the member's path; it ends as the worst of them (2 a tool failed, 1 new or resolved, 0 clean). Inside a member it runs that member alone."
)]
pub struct CheckArguments {
    /// Judge against what REV's branch accepted, so only what this branch introduced since REV is new; the lock is
    /// never written.
    #[arg(long, value_name = "REV")]
    pub(super) since: Option<String>,
    /// Run each tool's fix command first, saying on stderr each one that fails; the lock is never written.
    #[arg(long)]
    pub(super) fix: bool,
    /// Run only the tools of these groups (the labels the tools carry), comma separated; with none, every tool runs.
    #[arg(long, value_name = "GROUP", value_delimiter = ',')]
    pub(super) groups: Vec<String>,
    /// Run the groups the root asymptote.toml's `[hooks]` maps this git hook to, as the hook `asymptote init`
    /// installs does; not with `--groups`.
    #[arg(long, value_name = "HOOK", conflicts_with = "groups")]
    pub(super) hook: Option<String>,
}

/// Checks the arguments, composes the products, calls each once and shapes their answer, with the outcome the answer
/// ends the command as; when it cannot answer, the error is said on stderr and its outcome returned. The tools it runs
/// are registered with `on_signal`, which stops them when a SIGINT or SIGTERM ends the command.
pub(super) fn answer(
    arguments: CheckArguments,
    edge: &crate::edge::Edge,
    on_signal: fn(Box<dyn Fn() + Send>),
) -> Result<(impl crate::output::Encoded + use<>, Outcome), Outcome> {
    let hook = arguments.hook.clone();
    let request = super::schemas::request(arguments.since, arguments.fix, arguments.groups, edge)
        .map_err(|error| crate::output::fail(&error, Outcome::Failure))?;
    let refused = |error: asymptote_core::outbound::manifest::errors::ManifestError| {
        crate::output::fail(
            &asymptote_core::inbound::use_cases::check::errors::CheckError::from(error),
            Outcome::Error,
        )
    };
    // The root's `[hooks]` is read on every check, so a hook mapping a group no tool declares is refused before a
    // commit's hook meets it.
    let table = super::composition::build_hooks(&request.start).map_err(refused)?;
    let request = super::schemas::hooked_request(request, hook.as_deref(), Some(&table))
        .map_err(|error| crate::output::fail(&error, Outcome::Failure))?;
    let workspace = super::composition::build(
        &request.start,
        &super::connections::connect(&edge.build, &edge.store),
        on_signal,
    )
    .map_err(refused)?;
    let declared = super::composition::build_declared(&workspace).map_err(refused)?;
    super::schemas::known(&request, hook.as_deref(), &declared)
        .map_err(|error| crate::output::fail(&error, Outcome::Failure))?;
    super::schemas::layout(workspace.layout, edge.format)
        .map_err(|error| crate::output::fail(&error, Outcome::Failure))?;
    let checked: Vec<(String, _)> = workspace
        .products
        .iter()
        .map(|product| {
            (
                product.path.0.to_string(),
                product.service.check(CheckRequest {
                    start: product.root.clone(),
                    ..request.clone()
                }),
            )
        })
        .collect();
    // A SARIF log is one product's, rendered here: the format is the edge's choice, never the core's.
    let sarif = super::schemas::sarif_report(edge.format, &checked).map(|report| {
        use asymptote_core::outbound::sarif::interface::SarifPort as _;
        let runs: Vec<_> = report.runs.iter().map(|run| run.run.clone()).collect();
        let log = super::composition::build_sarif().write(&report.root, &runs, &report.outcome);
        log.0.to_string()
    });
    super::schemas::answered(workspace.layout, checked, sarif)
}

/// Runs `check`: runs the tools, judges them against the lock, prints each finding and the summary in the format
/// `--format` chose; ends as a failure when something is new or resolved, and as an error when a tool failed.
pub fn run(
    arguments: CheckArguments,
    edge: &crate::edge::Edge,
    on_signal: fn(Box<dyn Fn() + Send>),
) -> Outcome {
    match answer(arguments, edge, on_signal) {
        Ok(answered) => crate::output::present(edge.format, &answered.0, answered.1),
        Err(outcome) => outcome,
    }
}
