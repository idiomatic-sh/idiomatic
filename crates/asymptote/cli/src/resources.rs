//! The command tree: one resource, `ratchet`, whose commands are typed flat (`asymptote check`): the resource is the
//! place in the code, `resources/ratchet/<verb>/`, and is implicit on the command line. What the command line itself
//! answers is here too: help, the version, usage errors, `--no-color`, a panic, and SIGINT and SIGTERM, which end it
//! with every tool it runs.

pub mod ratchet;

/// The command line: a command of the ratchet.
#[derive(clap::Parser)]
#[command(
    name = "asymptote",
    about = "Ratchets what a product's linters report: accepted findings may only go down, and a new one fails"
)]
#[command(propagate_version = true, after_help = SUPPORT, arg = no_color())]
struct Cli {
    /// Where to look for the product's asymptote.toml: this directory, then its ancestors [default: the current directory].
    #[arg(long, global = true)]
    root: Option<std::path::PathBuf>,
    /// The `--timings` report every command takes.
    #[command(flatten)]
    timings: crate::timings::TimingsArgs,
    /// `--format` and its spellings: how the answer is printed.
    #[command(flatten)]
    output: crate::output::FormatSwitch,
    /// `-n/--dry-run`: say what the command would write instead of writing it.
    #[command(flatten)]
    dry_run: crate::output::dry_run::DryRunSwitch,
    /// The ratchet command the line names.
    #[command(subcommand)]
    command: ratchet::RatchetCommand,
}

/// The command tree, which the generated command-line contract test walks.
#[must_use]
pub fn command() -> clap::Command {
    // The format and `-n/--dry-run` are the command line's, taken anywhere on it.
    ["format", "json", "quiet", "dry_run"].into_iter().fold(
        <Cli as clap::CommandFactory>::command().version(env!("CARGO_PKG_VERSION")),
        |tree, id| tree.mut_arg(id, |argument| argument.global(true)),
    )
}

/// Parses the command line and runs the command it names, from `start` with `build`, handing `on_signal`, where a
/// command registers what stops the tools it runs, to the command that runs tools; a panic is reported as an internal
/// error and ends as one.
#[must_use]
pub fn run(
    start: &str,
    build: &str,
    on_signal: fn(Box<dyn Fn() + Send>),
) -> std::process::ExitCode {
    report_panics();
    let outcome = std::panic::catch_unwind(|| {
        run_from(std::env::args_os().collect(), start, build, on_signal)
    })
    .unwrap_or(crate::outcome::Outcome::Error);
    crate::output::progress::settle();
    std::process::ExitCode::from(outcome)
}

/// Runs the command line `arguments`, the program's name first.
fn run_from(
    arguments: Vec<std::ffi::OsString>,
    start: &str,
    build: &str,
    on_signal: fn(Box<dyn Fn() + Send>),
) -> crate::outcome::Outcome {
    let started = std::time::Instant::now();
    color(&arguments);
    crate::output::quiet(&arguments);
    let matches = match command().try_get_matches_from(arguments) {
        Ok(matches) => matches,
        Err(error) => return usage(&error),
    };
    let cli = match <Cli as clap::FromArgMatches>::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(error) => return usage(&error),
    };
    let _timings = match crate::timings::install(cli.timings.timings, started) {
        Ok(guard) => guard,
        Err(error) => return crate::output::fail(&error, crate::outcome::Outcome::Error),
    };
    let edge = crate::edge::Edge {
        start: asymptote_core::domain::manifest::types::ProductRoot(
            cli.root
                .map_or_else(
                    || start.to_owned(),
                    |root| std::path::Path::new(start).join(root).display().to_string(),
                )
                .into(),
        ),
        here: asymptote_core::domain::manifest::types::ProductRoot(start.into()),
        store: std::path::PathBuf::from(asymptote_core::domain::operations::constants::CACHE),
        build: asymptote_core::outbound::check_store::types::Build(build.into()),
        format: cli.output.format(),
        writing: if cli.dry_run.dry_run() {
            asymptote_core::domain::ratchet::types::Writing::DryRun
        } else {
            asymptote_core::domain::ratchet::types::Writing::Write
        },
    };
    ratchet::run(cli.command, &edge, on_signal)
}

/// Where to report a problem, printed under the top-level help.
const SUPPORT: &str = concat!(
    "Report a problem: ",
    env!("CARGO_PKG_REPOSITORY"),
    "/issues"
);

/// `--no-color`, which every command takes: no styles, wherever the output goes.
#[must_use]
fn no_color() -> clap::Arg {
    clap::Arg::new("no-color")
        .long("no-color")
        .global(true)
        .action(clap::ArgAction::SetTrue)
        .help("Print no styles, wherever the output goes, as `NO_COLOR` does")
}

/// Drops every style before anything is written when `arguments` hold `--no-color` before any `--`.
fn color(arguments: &[std::ffi::OsString]) {
    if arguments
        .iter()
        .take_while(|argument| *argument != "--")
        .any(|argument| argument == "--no-color")
    {
        anstream::ColorChoice::Never.write_global();
    }
}

/// Replaces Rust's panic message with what happened, where, and where to report it, on stderr.
fn report_panics() {
    std::panic::set_hook(Box::new(crate::output::panicked));
}

/// Prints clap's own answer, help, the version or a usage error, and ends as clap does.
fn usage(error: &clap::Error) -> crate::outcome::Outcome {
    let outcome = if error.use_stderr() {
        crate::outcome::Outcome::Error
    } else {
        crate::outcome::Outcome::Success
    };
    match error.print() {
        Err(failed) if failed.kind() != std::io::ErrorKind::BrokenPipe => {
            crate::outcome::Outcome::Error
        }
        _ => outcome,
    }
}
