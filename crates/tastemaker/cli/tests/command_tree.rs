//! The command tree of tm, held to what the Command Line Interface
//! Guidelines (clig.dev) ask of every command line, checked over its command tree, in
//! process. The `cli` spec's `command-tree-contract` template writes this file whole; a
//! product writes no line of it. It is a test target of its own, which Cargo finds
//! without a declaration. Its binary is held by `command-line-contract`.

/// The binary's name.
const PRODUCT: &str = "tm";

/// The one-letter flags a command line may take, each with the long it abbreviates
/// (clig.dev, "Arguments and flags": "Use standard names for flags").
const SHORTS: [(char, &str); 6] = [
    ('h', "help"),
    ('V', "version"),
    ('q', "quiet"),
    ('o', "output"),
    ('n', "dry-run"),
    ('f', "force"),
];

/// The commands that write, as the `cli` spec's `writes` names them: each takes
/// `-n/--dry-run`.
const WRITES: &[&str] = &["init", "add", "create", "scaffold", "update", "explain"];

/// The flag names that would carry a secret, which a flag may name only as the file that
/// holds it (clig.dev: "Do not read secrets directly from flags").
const SECRETS: [&str; 4] = ["password", "token", "secret", "key"];

/// The command tree as clap builds it, global flags propagated.
fn tree() -> clap::Command {
    let mut tree = tastemaker_cli::resources::command();
    tree.build();
    tree
}

/// The subcommands of `command`, without clap's own `help`.
fn children(command: &clap::Command) -> impl Iterator<Item = &clap::Command> {
    command
        .get_subcommands()
        .filter(|child| child.get_name() != "help")
}

/// Every command of the tree, the root first, each with its path below the root: the tree is flat, the one resource
/// `workspace` implicit on the command line.
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

/// clig.dev, "Help" and "Documentation": clap's own checks over the tree, and every
/// command and every flag says what it is in its help.
#[test]
fn the_tree_passes_claps_checks_and_documents_itself() {
    tastemaker_cli::resources::command().debug_assert();
    let tree = tree();
    for (path, command) in commands(&tree) {
        let at = path.join(" ");
        assert!(
            command.get_about().is_some(),
            "`{PRODUCT} {at}` says what it does"
        );
        for argument in command.get_arguments() {
            assert!(
                argument.get_help().is_some() || argument.get_long_help().is_some(),
                "`{PRODUCT} {at}`: `{}` says what it is",
                argument.get_id()
            );
        }
    }
}

/// clig.dev, "Naming": "Use only lowercase letters, and dashes if you really need to."
#[test]
fn the_binary_is_named_after_the_product() {
    assert_eq!(tree().get_name(), PRODUCT);
    assert!(
        PRODUCT.starts_with(|letter: char| letter.is_ascii_lowercase())
            && PRODUCT.chars().all(|letter| letter.is_ascii_lowercase()
                || letter.is_ascii_digit()
                || letter == '-'),
        "`{PRODUCT}` is lowercase letters, digits and dashes"
    );
}

/// The flat tree: each command is typed `tastemaker <verb>`, the resource `workspace` implicit, and
/// no command holds a subcommand.
#[test]
fn every_command_is_a_verb_with_no_subcommand() {
    let tree = tree();
    for command in children(&tree) {
        assert!(
            children(command).next().is_none(),
            "`{PRODUCT} {}` is a command, with no subcommand",
            command.get_name()
        );
    }
}

/// clig.dev, "Subcommands": "Don't have ambiguous or similarly-named commands"; and
/// "Future-proofing": "Don't have a catch-all subcommand". A name and its plural are the
/// one pair of siblings that may share a prefix: the plural lists what the singular
/// describes one of (`operations`, `operation <name>`).
#[test]
fn no_command_is_ambiguous_aliased_or_a_catch_all() {
    let tree = tree();
    for (path, command) in commands(&tree) {
        let at = path.join(" ");
        assert!(
            !command.is_allow_external_subcommands_set(),
            "`{PRODUCT} {at}` takes no unknown subcommand"
        );
        assert_eq!(
            command.get_all_aliases().count(),
            0,
            "`{PRODUCT} {at}` has one name, no alias"
        );
        let names: Vec<&str> = children(command).map(clap::Command::get_name).collect();
        for name in &names {
            for other in &names {
                assert!(
                    name == other || !other.starts_with(name) || *other == format!("{name}s"),
                    "`{PRODUCT} {at}`: `{name}` is a prefix of its sibling `{other}`"
                );
            }
        }
    }
}

/// clig.dev, "Arguments and flags": "Have full-length versions of all flags", "Only use
/// one-letter flags for commonly used flags", "Use standard names for flags".
#[test]
fn every_flag_has_a_long_and_only_a_standard_short() {
    let tree = tree();
    for (path, command) in commands(&tree) {
        let at = path.join(" ");
        for argument in command
            .get_arguments()
            .filter(|argument| !argument.is_positional())
        {
            let id = argument.get_id();
            let long = argument.get_long();
            assert!(long.is_some(), "`{PRODUCT} {at}`: `{id}` has a long name");
            assert!(
                argument.get_all_aliases().is_none(),
                "`{PRODUCT} {at}`: `{id}` has one long name"
            );
            assert!(
                argument.get_all_short_aliases().is_none(),
                "`{PRODUCT} {at}`: `{id}` has one short name"
            );
            if let Some(short) = argument.get_short() {
                assert!(
                    SHORTS.contains(&(short, long.unwrap_or_default())),
                    "`{PRODUCT} {at}`: `-{short}` abbreviates its standard long, not `--{}`",
                    long.unwrap_or_default()
                );
            }
            for (short, standard) in SHORTS {
                if long == Some(standard) {
                    assert_eq!(
                        argument.get_short(),
                        Some(short),
                        "`{PRODUCT} {at}`: `--{standard}` is also `-{short}`"
                    );
                }
            }
            if let Some(long) = long {
                for secret in SECRETS {
                    assert!(
                        !long.split('-').any(|word| word == secret) || long.ends_with("-file"),
                        "`{PRODUCT} {at}`: `--{long}` takes a secret; take the file that holds it, `--{long}-file`"
                    );
                }
            }
        }
    }
}

/// clig.dev, "Arguments and flags": "Prefer flags to args", "If you've got two or more
/// arguments for different things, you're probably doing something wrong": one positional a
/// command needs at most; a command may narrow with optional ones (`list [<kind>] [<term>]`).
#[test]
fn every_command_takes_at_most_one_required_positional() {
    let tree = tree();
    for (path, command) in commands(&tree) {
        let positionals: Vec<&str> = command
            .get_positionals()
            .filter(|argument| argument.is_required_set())
            .map(|argument| argument.get_id().as_str())
            .collect();
        assert!(
            positionals.len() <= 1,
            "`{PRODUCT} {}` needs {positionals:?}: one required positional at most, every other value a flag",
            path.join(" ")
        );
    }
}

/// clig.dev, "Arguments and flags": "make arguments, flags and subcommands
/// order-independent": a flag of the root is taken anywhere.
#[test]
fn every_root_flag_is_global() {
    let tree = tree();
    for argument in tree.get_arguments() {
        let id = argument.get_id().as_str();
        if id != "help" && id != "version" {
            assert!(argument.is_global_set(), "`{PRODUCT}`: `--{id}` is global");
        }
    }
}

/// clig.dev, "Output": "Display output as formatted JSON if `--json` is passed", "you can
/// provide a `-q` option"; "Subcommands": "Use the same flag names for the same things".
#[test]
fn every_command_takes_the_format_switch_json_quiet_and_no_color() {
    let tree = tree();
    for (path, command) in leaves(&tree) {
        let longs: Vec<&str> = command
            .get_arguments()
            .filter_map(clap::Arg::get_long)
            .collect();
        for long in ["format", "json", "quiet", "no-color"] {
            assert!(
                longs.contains(&long),
                "`{PRODUCT} {}` takes `--{long}`",
                path.join(" ")
            );
        }
    }
}

/// clig.dev, "Help": "Lead with examples", and concise help has "One or two example
/// invocations".
#[test]
fn every_command_leads_with_an_example() {
    let tree = tree();
    for (path, command) in leaves(&tree) {
        let invocation = format!("{PRODUCT} {}", path.join(" "));
        let help = command
            .get_after_help()
            .map(ToString::to_string)
            .unwrap_or_default();
        assert!(
            help.contains("Examples:")
                && help
                    .lines()
                    .any(|line| line.trim_start().starts_with(&invocation)),
            "`{invocation}`'s help ends with `Examples:` and a line starting `{invocation}`"
        );
    }
}

/// clig.dev, "Arguments and flags": "Confirm before doing anything dangerous ... consider
/// ... `--dry-run`": every command that writes takes `-n/--dry-run`, its own or the command
/// line's global one.
#[test]
fn every_command_that_writes_takes_dry_run() {
    let tree = tree();
    let leaves = leaves(&tree);
    for (path, command) in &leaves {
        let at = path.join(" ");
        let dry_run = command
            .get_arguments()
            .any(|argument| argument.get_long() == Some("dry-run"));
        assert!(
            dry_run || !WRITES.contains(&at.as_str()),
            "`{PRODUCT} {at}` writes, as the `cli` spec's `writes` names it, so it takes `--dry-run`"
        );
    }
    for written in WRITES {
        assert!(
            leaves.iter().any(|(path, _)| path.join(" ") == *written),
            "`writes` names `{written}`, which is no command of `{PRODUCT}`"
        );
    }
}

/// clig.dev, "Future-proofing": "Keep changes additive", "Warn before you make a
/// non-additive change": every command, flag, short and value name, one line each,
/// equals the checked-in `tests/command_tree/surface.txt`, so a change to the surface is
/// a visible change to that file (`SNAPSHOTS=overwrite` rewrites it).
#[test]
fn the_command_surface_is_the_one_checked_in() {
    snapbox::assert_data_eq!(surface(&tree()), snapbox::file!["command_tree/surface.txt"]);
}

/// The surface of `tree`: each command's path, then each of its arguments under it,
/// `--long -s <VALUE>` or `<POSITIONAL>`, help and version left out.
fn surface(tree: &clap::Command) -> String {
    let mut lines = Vec::new();
    for (path, command) in commands(tree) {
        let at = std::iter::once(PRODUCT)
            .chain(path.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(at);
        let mut arguments: Vec<String> = command
            .get_arguments()
            .filter(|argument| {
                !argument.is_hide_set()
                    && !["help", "version"].contains(&argument.get_id().as_str())
            })
            .map(|argument| {
                let values = argument
                    .get_value_names()
                    .filter(|_| argument.get_action().takes_values())
                    .map(|names| {
                        names
                            .iter()
                            .map(|name| [" <", name.as_str(), ">"].concat())
                            .collect::<String>()
                    })
                    .unwrap_or_default();
                match (argument.get_long(), argument.get_short()) {
                    (Some(long), Some(short)) => format!("  --{long} -{short}{values}"),
                    (Some(long), None) => format!("  --{long}{values}"),
                    (None, _) => argument
                        .get_value_names()
                        .and_then(<[clap::builder::Str]>::first)
                        .map_or_else(
                            || format!("  <{}>", argument.get_id()),
                            |name| format!("  <{name}>"),
                        ),
                }
            })
            .collect();
        arguments.sort();
        lines.extend(arguments);
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}
