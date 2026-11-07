//! The workspace every workload runs over: one package `p` with a rule, a spec and its template, each with the
//! scenarios that `validate` demands, and a source tree to place and check.

/// The rule of the bench workspace: a comment `// MARKER` is a finding.
const RULE: &str = r#"description = "A marker comment."
why = "A bench needs a rule."
query = '''
let found = select * from "comment" where text = " MARKER";
emit found subject "file" key("span");
'''

[diagnostic]
primary = "span"
message = "a marker"
help = "Remove it."

[[scenarios]]
id = 'clean'
scenario = 'A file without the marker.'
given.include = ['**']
given.files.'src/a/errors.rs' = '''
pub enum Error {}
'''

[[scenarios]]
id = 'marked'
scenario = 'A file with the marker.'
given.include = ['**']
given.files.'src/a/errors.rs' = '''
// MARKER
pub enum Error {}
'''
then.findings = [
  { path = 'src/a/errors.rs', line = 1 },
]
"#;

/// The spec of the bench workspace: one `errors.rs` in each `src/<domain>`.
pub(crate) const SPEC: &str = r#"description = "A part."
why = "A bench needs a spec."
templates = [{ template = "errors", file = "errors.rs", description = "The errors." }]

[[scenarios]]
id = 'complete'
scenario = 'A part with its errors.'
given.at = 'src/a'
given.files.'src/a/errors.rs' = '''
pub enum Error {}
'''

[[scenarios]]
id = 'missing'
scenario = 'A part without its errors.'
given.at = 'src/a'
then.findings = [
  { rule = 'p/part#count', path = 'tastemaker.toml' },
]
"#;

/// The template of the spec.
pub(crate) const TEMPLATE: &str = r#"description = "The errors."
why = "A bench needs a template."
content = "pub enum Error {}\n"

[[scenarios]]
id = 'rendered'
scenario = 'The rendered template passes.'
given.file = 'errors.rs'
given.files.'errors.rs' = '''
pub enum Error {}
'''

[[scenarios]]
id = 'missing'
scenario = 'The file is missing.'
given.file = 'errors.rs'
then.findings = [
  { rule = 'errors#count', path = '.' },
]
"#;

/// A workspace in a temporary directory, a git repository holding one package: a rule that finds a marker comment, a
/// spec that places one `errors.rs` in each `src/<domain>`; its lock written by a first `check --fix`.
///
/// # Panics
///
/// When the directory cannot be written.
#[expect(
    clippy::expect_used,
    reason = "a benchmark's setup fails by panicking: a scene that cannot be written or a line that cannot be parsed is a broken benchmark, and the message names which"
)]
pub(crate) fn workspace() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let files = [
        (
            "tastemaker.toml",
            "workspace = { name = \"w\", dependencies = [{ package = \"p\", path = \"p\" }] }\nclosed = false\nrules = [{ rule = \"p/marker\", include = [\"**\"] }]\nspecs = [{ spec = \"p/part\", include = [\"src/{domain}\"] }]\n",
        ),
        ("p/package.toml", "name = \"p\"\n"),
        ("p/rules/marker/0.1.0/rule.toml", RULE),
        ("p/specs/part/0.1.0/spec.toml", SPEC),
        (
            "p/specs/part/0.1.0/templates/errors/template.toml",
            TEMPLATE,
        ),
        ("src/a/errors.rs", "// MARKER\npub enum Error {}\n"),
        ("src/b/errors.rs", "pub enum Error {}\n"),
    ];
    std::fs::create_dir_all(directory.path().join(".git")).expect("a .git directory");
    for (path, text) in files {
        let place = directory.path().join(path);
        std::fs::create_dir_all(place.parent().expect("a parent")).expect("its directory");
        std::fs::write(place, text).expect("the file");
    }
    let edge = edge(&directory);
    assert_eq!(
        tastemaker_cli::resources::workspace::validate::command::run(
            arguments(&["validate"]),
            &edge
        ),
        tastemaker_cli::outcome::Outcome::Success,
        "the workload package validates"
    );
    assert_eq!(
        tastemaker_cli::resources::workspace::check::command::run(
            arguments(&["check", "--fix"]),
            &edge,
        ),
        tastemaker_cli::outcome::Outcome::Failure,
        "the workload finds its marker"
    );
    directory
}

/// The edge of a command run in `directory`, its answer printed nowhere (`-q`).
pub(crate) fn edge(directory: &tempfile::TempDir) -> tastemaker_cli::edge::Edge {
    tastemaker_cli::resources::edge_at(
        tastemaker_core::domain::files::types::MachinePath(
            directory.path().display().to_string().into(),
        ),
        tastemaker_core::domain::settings::types::Build("bench".into()),
        tastemaker_cli::output::Format::Quiet,
        tastemaker_core::domain::writing::types::Writing::Write,
    )
}

/// `edge` with `-n/--dry-run` given: the command writes nothing.
pub(crate) fn dry(edge: tastemaker_cli::edge::Edge) -> tastemaker_cli::edge::Edge {
    tastemaker_cli::edge::Edge {
        writing: tastemaker_core::domain::writing::types::Writing::DryRun,
        ..edge
    }
}

/// A command's arguments, parsed from `line`, the command's name first.
///
/// # Panics
///
/// When the line does not parse.
#[expect(
    clippy::expect_used,
    reason = "a benchmark's setup fails by panicking: a scene that cannot be written or a line that cannot be parsed is a broken benchmark, and the message names which"
)]
pub(crate) fn arguments<A>(line: &[&str]) -> A
where
    A: clap::Args + clap::FromArgMatches,
{
    let command = A::augment_args(clap::Command::new("bench"));
    let matches = command
        .try_get_matches_from(line)
        .expect("the bench's command line parses");
    A::from_arg_matches(&matches).expect("the bench's arguments")
}
