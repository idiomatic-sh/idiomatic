//! The command tree: one resource, `workspace`, whose commands are typed flat (`tm check`): the resource is the
//! place in the code, `resources/workspace/<verb>/`, and is implicit on the command line. What the command line itself
//! answers is here too: help, the version with the languages linked, usage errors, `--no-color` and a panic.

pub mod workspace;

/// The directory beneath a project's root where Tastemaker keeps its store: written here once, and handed to every
/// adapter that must know it, the walks that skip it and the stores that keep their files in it.
#[must_use]
fn store() -> tastemaker_core::domain::files::types::DirectoryName {
    tastemaker_core::domain::files::types::DirectoryName(".tastemaker_cache".into())
}

/// The edge of a command run in `start`, which is also where it was started: the one place besides `run_from` that
/// builds an [`Edge`](crate::edge::Edge), so the store's name stays written once.
#[must_use]
pub fn edge_at(
    start: tastemaker_core::domain::files::types::MachinePath,
    build: tastemaker_core::domain::settings::types::Build,
    format: crate::output::Format,
    writing: tastemaker_core::domain::writing::types::Writing,
) -> crate::edge::Edge {
    crate::edge::Edge {
        here: start.clone(),
        start,
        store: store(),
        build,
        format,
        writing,
    }
}

/// The command line: a command of the workspace.
#[derive(clap::Parser)]
#[command(
    name = "tm",
    about = "Places a workspace's files by its tastemaker.toml and checks them with ryo"
)]
#[command(propagate_version = true, after_help = SUPPORT, arg = no_color())]
struct Cli {
    /// Where to look for the workspace: this directory, then its ancestors [default: the current directory].
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
    /// The workspace command the line names.
    #[command(subcommand)]
    command: workspace::WorkspaceCommand,
}

/// What `-V` prints: the version, then the languages this build links.
fn version() -> &'static str {
    static TEXT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TEXT.get_or_init(|| {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            crate::languages::described()
        )
    })
}

/// The command tree, which the generated command-line contract test walks.
#[must_use]
pub fn command() -> clap::Command {
    // The format and `-n/--dry-run` are the command line's, taken anywhere on it (the decided command tree).
    ["format", "json", "quiet", "dry_run"].into_iter().fold(
        <Cli as clap::CommandFactory>::command().version(version()),
        |tree, id| tree.mut_arg(id, |argument| argument.global(true)),
    )
}

/// Parses the command line and runs the command it names, from `start` with `build`; a panic is reported as an internal
/// error and ends as one.
#[must_use]
pub fn run(start: &str, build: &str) -> std::process::ExitCode {
    report_panics();
    let outcome =
        std::panic::catch_unwind(|| run_from(std::env::args_os().collect(), start, build))
            .unwrap_or(crate::outcome::Outcome::Error);
    crate::output::progress::settle();
    std::process::ExitCode::from(outcome)
}

/// Runs the command line `arguments`, the program's name first.
fn run_from(
    arguments: Vec<std::ffi::OsString>,
    start: &str,
    build: &str,
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
    let _timings = crate::timings::install(cli.timings.timings, started);
    let here = tastemaker_core::domain::files::types::MachinePath(start.into());
    let edge = crate::edge::Edge {
        start: cli.root.map_or_else(
            || here.clone(),
            |root| {
                tastemaker_core::domain::files::types::MachinePath(
                    root.display().to_string().into(),
                )
            },
        ),
        here,
        store: store(),
        build: tastemaker_core::domain::settings::types::Build(build.into()),
        format: cli.output.format(),
        writing: if cli.dry_run.dry_run() {
            tastemaker_core::domain::writing::types::Writing::DryRun
        } else {
            tastemaker_core::domain::writing::types::Writing::Write
        },
    };
    workspace::run(cli.command, &edge)
}

/// Where to report a problem, printed under the top-level help.
const SUPPORT: &str = concat!(
    "Get started: tm init\n",
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
