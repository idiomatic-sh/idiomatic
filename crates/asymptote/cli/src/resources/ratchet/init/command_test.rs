//! What `init` promises its caller, one row of `CASES` for each case it has, over a directory on disk.

use super::command::{InitArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;
use crate::resources::ratchet::check::fixtures::{chosen, edge};

/// What the directory is.
#[derive(Clone, Copy, Debug)]
enum Directory {
    /// A fresh git repository.
    Repository,
    /// A git repository with an `asymptote.toml` of its own.
    Configured,
    /// A git repository whose `pre-commit` is someone else's.
    Foreign,
    /// A directory in no git repository.
    Plain,
    /// A fresh git repository, where `init` runs in a directory beneath its top level.
    Subdirectory,
    /// No run: what `init`'s help says a root `asymptote.toml` without `[hooks]` installs.
    Help,
}

/// The command line, after `init`, and what it runs in.
type Input = (Vec<&'static str>, Directory);
/// What the answer says.
type Value = std::borrow::Cow<'static, str>;
/// How the command ends when it cannot answer.
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    /// The arguments the line parses into.
    #[command(flatten)]
    arguments: InitArguments,
    /// The format the line chose, which the command tree reads into the edge.
    #[command(flatten)]
    output: crate::output::FormatSwitch,
    /// The dry run the line chose, which the command tree reads into the edge.
    #[command(flatten)]
    dry_run: crate::output::dry_run::DryRunSwitch,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_fresh_repository_gets_a_configuration_and_the_default_hook_then_the_next_steps",
        ),
        word: Word::Succeeds,
        input: || (Vec::new(), Directory::Repository),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "wrote asymptote.toml and an empty lock\nwrote the pre-commit hook\nnext: asymptote check                      run the tools against the lock\nnext: asymptote up --reason <why>          accept what they find now, once\n",
        )),
    },
    Case {
        name: CaseName("a_dry_run_says_what_it_would_write_and_no_next_steps"),
        word: Word::Succeeds,
        input: || (vec!["-n"], Directory::Repository),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "would write asymptote.toml and an empty lock\nwould write the pre-commit hook\n",
        )),
    },
    Case {
        name: CaseName("an_existing_configuration_is_kept_and_said_so"),
        word: Word::Succeeds,
        input: || (Vec::new(), Directory::Configured),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "kept asymptote.toml: it is there, and is left as it is\nwrote the pre-commit hook\nnext: asymptote check                      run the tools against the lock\nnext: asymptote up --reason <why>          accept what they find now, once\n",
        )),
    },
    Case {
        name: CaseName("run_beneath_the_top_level_it_writes_the_configuration_at_the_top_level"),
        word: Word::Succeeds,
        input: || (Vec::new(), Directory::Subdirectory),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "wrote asymptote.toml and an empty lock\nwrote the pre-commit hook\nnext: asymptote check                      run the tools against the lock\nnext: asymptote up --reason <why>          accept what they find now, once\nat the top level: true; beneath it: false\n",
        )),
    },
    Case {
        name: CaseName("the_help_says_a_configuration_without_hooks_installs_none"),
        word: Word::Succeeds,
        input: || (Vec::new(), Directory::Help),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "none when it has no table; the asymptote.toml it writes maps `pre-commit` to the lint group",
        )),
    },
    Case {
        name: CaseName("a_hook_that_is_not_asymptotes_is_refused_as_a_failure"),
        word: Word::Refuses,
        input: || (Vec::new(), Directory::Foreign),
        expect: Expect::error(Outcome::Failure),
    },
    Case {
        name: CaseName("a_directory_in_no_repository_is_refused_as_a_failure"),
        word: Word::Refuses,
        input: || (Vec::new(), Directory::Plain),
        expect: Expect::error(Outcome::Failure),
    },
];

/// Makes `directory` a fresh git repository.
fn git(directory: &std::path::Path) {
    let made = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(directory)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .expect("git runs");
    assert!(made.status.success(), "`git init` makes the repository");
}

/// The directory on disk the row runs in, and the edge that starts `init` there or beneath it.
fn scene(directory: Directory) -> (tempfile::TempDir, crate::edge::Edge) {
    let product = tempfile::tempdir().expect("a temporary directory");
    let path = product.path();
    if !matches!(directory, Directory::Plain) {
        git(path);
    }
    match directory {
        Directory::Configured => std::fs::write(
            path.join("asymptote.toml"),
            "[hooks]\npre-commit = [\"lint\"]\n",
        )
        .expect("the configuration is written"),
        Directory::Foreign => {
            std::fs::write(
                path.join(".git/hooks/pre-commit"),
                "#!/bin/sh\necho theirs\n",
            )
            .expect("the foreign hook is written");
        }
        Directory::Repository | Directory::Plain | Directory::Subdirectory | Directory::Help => {}
    }
    let edge = if matches!(directory, Directory::Subdirectory) {
        let beneath = path.join("packages/a");
        std::fs::create_dir_all(&beneath).expect("the directory beneath the top level is made");
        crate::edge::Edge {
            start: asymptote_core::domain::manifest::types::ProductRoot(
                beneath.display().to_string().into(),
            ),
            ..edge(&product)
        }
    } else {
        edge(&product)
    };
    (product, edge)
}

/// What `init`'s help says, in its parenthesis, of a root `asymptote.toml` without `[hooks]`.
fn help() -> String {
    let tree = crate::resources::command();
    let help = tree
        .find_subcommand("init")
        .and_then(clap::Command::get_after_help)
        .map(ToString::to_string)
        .unwrap_or_default();
    help.split_once("`[hooks]` table (")
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(said, _)| said.to_owned())
        .unwrap_or_default()
}

/// Parses the row's command line, which every row writes to parse.
fn parsed(line: Vec<&'static str>) -> Line {
    <Line as clap::Parser>::try_parse_from(std::iter::once("init").chain(line))
        .expect("the row's command line parses")
}

/// Runs `init` with the row's command line in its directory; beneath the top level, also where it wrote.
fn subject((line, directory): Input) -> Result<Value, Failure> {
    if matches!(directory, Directory::Help) {
        return Ok(std::borrow::Cow::Owned(help()));
    }
    let (product, edge) = scene(directory);
    let line = parsed(line);
    let edge = chosen(edge, line.output, line.dry_run);
    let said = answer(line.arguments, &edge)?.to_string();
    let path = product.path();
    Ok(std::borrow::Cow::Owned(match directory {
        Directory::Subdirectory => format!(
            "{said}at the top level: {}; beneath it: {}\n",
            path.join("asymptote.toml").is_file(),
            path.join("packages/a").join("asymptote.toml").is_file(),
        ),
        Directory::Repository
        | Directory::Configured
        | Directory::Foreign
        | Directory::Plain
        | Directory::Help => said,
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
