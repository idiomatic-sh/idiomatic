//! The command line of asymptote, held to what the Command Line Interface
//! Guidelines (clig.dev) ask of every command line, checked over its built binary. The
//! `cli` spec's `command-line-contract` template writes this file whole; a product writes no
//! line of it. It is a test target of its own, which Cargo finds without a declaration. Its
//! command tree is held by `command-tree-contract`.

use std::process::{Command, Output};

/// The binary's name.
const PRODUCT: &str = "asymptote";

/// The command tree as clap builds it, global flags propagated.
fn tree() -> clap::Command {
    let mut tree = asymptote_cli::resources::command();
    tree.build();
    tree
}

/// The subcommands of `command`, without clap's own `help`.
fn children(command: &clap::Command) -> impl Iterator<Item = &clap::Command> {
    command
        .get_subcommands()
        .filter(|child| child.get_name() != "help")
}

/// Every command of the tree, the root first, each with its path below the root: the tree is flat.
fn commands(tree: &clap::Command) -> Vec<(Vec<String>, &clap::Command)> {
    let mut found = vec![(Vec::new(), tree)];
    for command in children(tree) {
        found.push((vec![command.get_name().to_owned()], command));
    }
    found
}

/// The commands that run: every `<command>`.
fn leaves(tree: &clap::Command) -> Vec<(Vec<String>, &clap::Command)> {
    commands(tree)
        .into_iter()
        .filter(|(path, _)| path.len() == 1)
        .collect()
}

/// The binary run with `arguments`, with no colour variable of the caller's.
fn run(arguments: &[&str]) -> std::io::Result<Output> {
    run_with(arguments, &[])
}

/// The binary run with `arguments` and the colour `variables` set on it alone.
fn run_with(arguments: &[&str], variables: &[(&str, &str)]) -> std::io::Result<Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_asymptote"));
    ["NO_COLOR", "CLICOLOR", "CLICOLOR_FORCE", "TERM"]
        .into_iter()
        .fold(command.args(arguments), |unset, variable| {
            unset.env_remove(variable)
        })
        .envs(variables.iter().copied())
        .output()
}

/// `output`'s stream as text.
fn text(stream: &[u8]) -> String {
    String::from_utf8_lossy(stream).into_owned()
}

/// clig.dev, "Help": "Provide a support path for feedback and issues. A website or GitHub
/// link in the top-level help text is common."
#[test]
fn the_top_level_help_says_where_to_report_a_problem() {
    let repository = env!("CARGO_PKG_REPOSITORY");
    assert!(
        repository.starts_with("https://"),
        "the package names its repository"
    );
    let help = run(&["--help"]).unwrap();
    assert!(text(&help.stdout).contains(&format!("{repository}/issues")));
}

/// clig.dev, "Help": "Display help when passed `-h` or `--help` flags. This also applies to
/// subcommands", and `help <command>`.
#[test]
fn every_command_answers_help() {
    let tree = tree();
    for (path, _) in commands(&tree) {
        let path: Vec<&str> = path.iter().map(String::as_str).collect();
        let at = path.join(" ");
        let helped = [
            [path.as_slice(), &["--help"]].concat(),
            [path.as_slice(), &["-h"]].concat(),
            [&["help"], path.as_slice()].concat(),
        ];
        for arguments in helped {
            let output = run(&arguments).unwrap();
            assert_eq!(
                output.status.code(),
                Some(0_i32),
                "`{PRODUCT} {}` exits 0",
                arguments.join(" ")
            );
            assert!(
                !output.stdout.is_empty(),
                "`{PRODUCT} {}` prints help on stdout",
                arguments.join(" ")
            );
            assert!(
                output.stderr.is_empty(),
                "`{PRODUCT} {}` prints nothing on stderr",
                arguments.join(" ")
            );
            assert!(
                !output.stdout.contains(&0x1b),
                "`{PRODUCT} {}` piped is not styled",
                arguments.join(" ")
            );
        }
        let short = run(&[path.as_slice(), &["-h"]].concat()).unwrap();
        assert!(
            text(&short.stdout).contains("Usage:"),
            "`{PRODUCT} {at} -h` shows the usage"
        );
    }
}

/// clig.dev, "Help": "Display concise help text by default" when a command needs more to
/// run: on stderr, as a usage error.
#[test]
fn a_command_line_that_names_no_command_prints_help_and_fails() {
    // The tree is flat: the root alone names no command; each command runs as it is.
    let bare: Vec<Vec<&str>> = vec![Vec::new()];
    for arguments in bare {
        let output = run(&arguments).unwrap();
        assert_eq!(
            output.status.code(),
            Some(2_i32),
            "`{PRODUCT} {}` exits 2",
            arguments.join(" ")
        );
        assert!(
            output.stdout.is_empty(),
            "`{PRODUCT} {}` prints nothing on stdout",
            arguments.join(" ")
        );
        assert!(
            text(&output.stderr).contains("Usage:"),
            "`{PRODUCT} {}` prints help on stderr",
            arguments.join(" ")
        );
    }
}

/// clig.dev, "Arguments and flags": the standard `--version`, on every command.
#[test]
fn every_command_answers_its_version() {
    let tree = tree();
    // The version alone: `asymptote 0.1.0`, which provisioning compares with the workspace's.
    let version = format!(" {}", env!("CARGO_PKG_VERSION"));
    for (path, _) in commands(&tree) {
        let path: Vec<&str> = path.iter().map(String::as_str).collect();
        for flag in ["--version", "-V"] {
            let arguments = [path.as_slice(), &[flag]].concat();
            let at = arguments.join(" ");
            let output = run(&arguments).unwrap();
            let said = text(&output.stdout);
            assert_eq!(
                output.status.code(),
                Some(0_i32),
                "`{PRODUCT} {at}` exits 0"
            );
            assert!(
                said.starts_with(PRODUCT) && said.contains(&version),
                "`{PRODUCT} {at}` prints the product and its version: {said}"
            );
        }
    }
}

/// clig.dev, "The Basics": "Return zero exit code on success, non-zero on failure", "Send
/// messaging to `stderr`"; "Help": "suggest what command to run next".
#[test]
fn an_unknown_flag_is_a_usage_error_on_stderr() {
    let tree = tree();
    for (path, _) in leaves(&tree) {
        let path: Vec<&str> = path.iter().map(String::as_str).collect();
        let arguments = [path.as_slice(), &["--not-a-flag-of-any-command"]].concat();
        let output = run(&arguments).unwrap();
        let at = arguments.join(" ");
        assert_eq!(
            output.status.code(),
            Some(2_i32),
            "`{PRODUCT} {at}` exits 2"
        );
        assert!(
            output.stdout.is_empty(),
            "`{PRODUCT} {at}` prints nothing on stdout"
        );
        let said = text(&output.stderr);
        assert!(
            said.starts_with("error:"),
            "`{PRODUCT} {at}` says `error:` first: {said}"
        );
        assert!(
            said.contains("--help"),
            "`{PRODUCT} {at}` names `--help`: {said}"
        );
    }
}

/// clig.dev, "Help": "If the user did something wrong and you can guess what they meant,
/// suggest it"; "Future-proofing": "Don't allow arbitrary abbreviations of subcommands".
#[test]
fn a_mistyped_or_abbreviated_resource_is_refused_and_named() {
    let tree = tree();
    let names: Vec<&str> = children(&tree).map(clap::Command::get_name).collect();
    for name in &names {
        let mistyped = format!("{name}x");
        let output = run(&[&mistyped]).unwrap();
        assert_eq!(
            output.status.code(),
            Some(2_i32),
            "`{PRODUCT} {mistyped}` exits 2"
        );
        assert!(
            text(&output.stderr).contains(&format!("'{name}'")),
            "`{PRODUCT} {mistyped}` suggests `{name}`"
        );
        let abbreviated = name.strip_suffix(|_: char| true).unwrap_or_default();
        if !abbreviated.is_empty() && !names.contains(&abbreviated) {
            let refused = run(&[abbreviated, "--help"]).unwrap();
            assert_eq!(
                refused.status.code(),
                Some(2_i32),
                "`{PRODUCT} {abbreviated}` is not `{name}`"
            );
        }
    }
    for resource in children(&tree) {
        let verbs: Vec<&str> = children(resource).map(clap::Command::get_name).collect();
        for verb in &verbs {
            let abbreviated = verb.strip_suffix(|_: char| true).unwrap_or_default();
            if !abbreviated.is_empty() && !verbs.contains(&abbreviated) {
                let output = run(&[resource.get_name(), abbreviated, "--help"]).unwrap();
                assert_eq!(
                    output.status.code(),
                    Some(2_i32),
                    "`{PRODUCT} {} {abbreviated}` is not `{verb}`",
                    resource.get_name()
                );
            }
        }
    }
}

/// clig.dev, "Help": "Use formatting in your help text"; "Output": "Disable color if ...
/// The `NO_COLOR` environment variable is set ... The user passes the option
/// `--no-color`". The variables are anstream's and clap's to read: they are set on the
/// binary alone, and this test reads none.
#[test]
fn help_is_styled_unless_colour_is_turned_off() {
    let forced = [("CLICOLOR_FORCE", "1")];
    assert!(
        run_with(&["--help"], &forced)
            .unwrap()
            .stdout
            .contains(&0x1b),
        "`CLICOLOR_FORCE=1 {PRODUCT} --help` is styled"
    );
    let off = [("CLICOLOR_FORCE", "1"), ("NO_COLOR", "1")];
    assert!(
        !run_with(&["--help"], &off).unwrap().stdout.contains(&0x1b),
        "`NO_COLOR=1` turns styles off"
    );
    assert!(
        !run_with(&["--no-color", "--help"], &forced)
            .unwrap()
            .stdout
            .contains(&0x1b),
        "`--no-color` turns styles off"
    );
}
