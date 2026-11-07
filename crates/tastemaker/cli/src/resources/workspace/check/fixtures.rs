//! The test data of `workspace check`, compiled only for tests and under the `fixtures` feature.
#![cfg(any(test, feature = "fixtures"))]
#![cfg_attr(
    all(feature = "fixtures", not(test)),
    expect(
        dead_code,
        reason = "the feature compiles the commands' test data for the crates that run their rows; within this crate only its tests read it"
    )
)]
//! The test data of the commands: a workspace on disk, in a temporary directory of its own, holding one package.

/// The package's rule: a comment `# MARKER` is a finding, in a TOML file: nothing of it rests on a programming language.
const RULE: &str = r##"description = "A marker comment is reported."
why = "A test needs a rule that finds something."
query = '''
let found = select * from "comment" where text = " MARKER";
emit found subject "file" key("span");
'''

[diagnostic]
primary = "span"
message = "a marker"
help = "Remove the marker."

[[scenarios]]
id = "marker-in-a-line-comment"
scenario = "A marker comment is reported."
given.include = ["**"]
given.files."src/notes.toml" = "# MARKER\n"
then.findings = [{ path = "src/notes.toml", line = 1 }]

[[scenarios]]
id = "comment-without-a-marker"
scenario = "A comment that is no marker is not reported."
given.include = ["**"]
given.files."src/notes.toml" = "# other\n"
"##;

/// The package's spec: one `errors.toml` in each instance.
const SPEC: &str = r##"description = "A part: one errors.toml."
why = "A test needs a spec that places a file."
templates = [
  { template = "errors", file = "errors.toml", description = "The part's errors." },
]

[[scenarios]]
id = "part-with-its-errors"
scenario = "A part with its errors.toml has no finding."
given.at = "src/a"
given.files."src/a/errors.toml" = "errors = []\n"

[[scenarios]]
id = "part-with-a-stray-file"
scenario = "A part with a file no template places is reported."
given.at = "src/a"
given.files."src/a/errors.toml" = "errors = []\n"
given.files."src/a/stray.toml" = "stray = []\n"
then.findings = [{ rule = "#closed", path = "src/a/stray.toml" }]
"##;

/// The spec's template.
const TEMPLATE: &str = "description = \"The errors.\"\nwhy = \"A test needs a template.\"\ncontent = \"errors = []\\n\"\n\n[[scenarios]]\nid = \"errors-file-present\"\nscenario = \"The file is there.\"\ngiven.file = \"errors.toml\"\ngiven.files.\"errors.toml\" = \"errors = []\\n\"\n\n[[scenarios]]\nid = \"errors-file-missing\"\nscenario = \"The file is missing.\"\ngiven.file = \"errors.toml\"\nthen.findings = [{ rule = \"errors#count\", path = \".\" }]\n";

/// The workspace's file.
const WORKSPACE: &str = r#"workspace = { name = "w", dependencies = [{ package = "p", path = "p" }] }
closed = false
rules = [{ rule = "p/marker", include = ["**"] }]
specs = [{ spec = "p/part", include = ["src/{domain}"] }]
"#;

/// A workspace in a temporary directory, a git repository: the marker in `src/a/errors.toml`, `src/b` missing its file.
///
/// # Panics
///
/// When the directory cannot be written.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "test data: a workspace that cannot be written is a broken test, and the panic names the file"
)]
pub(crate) fn workspace() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let files = [
        ("tastemaker.toml", WORKSPACE),
        ("p/package.toml", "name = \"p\"\n"),
        ("p/rules/marker/0.1.0/rule.toml", RULE),
        ("p/specs/part/0.1.0/spec.toml", SPEC),
        (
            "p/specs/part/0.1.0/templates/errors/template.toml",
            TEMPLATE,
        ),
        ("src/a/errors.toml", "# MARKER\nerrors = []\n"),
        ("src/b/notes.toml", "b = []\n"),
    ];
    std::fs::create_dir_all(directory.path().join(".git")).expect("a .git directory");
    for (path, text) in files {
        let place = directory.path().join(path);
        std::fs::create_dir_all(place.parent().expect("a parent")).expect("its directory");
        std::fs::write(place, text).expect("the file");
    }
    directory
}

/// `edge` with `-n/--dry-run` given: what a command line with `-n` hands its command.
#[must_use]
pub(crate) fn dry_run(edge: crate::edge::Edge) -> crate::edge::Edge {
    crate::edge::Edge {
        writing: tastemaker_core::domain::writing::types::Writing::DryRun,
        ..edge
    }
}

/// The edge of a command run in `directory`.
#[must_use]
pub(crate) fn edge(directory: &tempfile::TempDir) -> crate::edge::Edge {
    crate::resources::edge_at(
        tastemaker_core::domain::files::types::MachinePath(
            directory.path().display().to_string().into(),
        ),
        tastemaker_core::domain::settings::types::Build("test".into()),
        crate::output::Format::Terminal,
        tastemaker_core::domain::writing::types::Writing::Write,
    )
}

/// Every file beneath `directory`, with its bytes; a directory or file that cannot be read is the error, never one with
/// nothing in it.
///
/// # Errors
///
/// When a directory cannot be listed or a file read.
pub(crate) fn files(
    directory: &std::path::Path,
) -> std::io::Result<std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>> {
    let mut found = std::collections::BTreeMap::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(at) = pending.pop() {
        for entry in std::fs::read_dir(&at)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = std::fs::read(&path)?;
                drop(found.insert(path, bytes));
            }
        }
    }
    Ok(found)
}

/// What the fixture's check finds with the templates of `template_sources` placed: a Rust module an extra decoration
/// disables, a Python and a TypeScript file each missing a function, a data document whose sequence is emptied and whose
/// second table lacks a key the first carries, and a text file no linked language reads.
#[cfg(test)]
pub(crate) const CONTRACTED: [&str; 9] = [
    "p/gate#template",
    "p/marker",
    "p/notes#unknown",
    "p/part#closed",
    "p/part/errors#count",
    "p/py-service#template",
    "p/tool-config#template",
    "p/tool-config#template",
    "p/ts-module#template",
];

/// The templates of `template_sources`: each its name, the file it places, its content, and the file as written.
#[cfg(test)]
const CONTRACTS: [(&str, &str, &str, &str); 5] = [
    (
        "gate",
        "c/gate.rs",
        "#[path = \"x.rs\"]\nmod login;\n",
        "#[cfg(any())]\n#[path = \"x.rs\"]\nmod login;\n",
    ),
    (
        "py-service",
        "c/service.py",
        "def create():\n    pass\n\n\ndef delete():\n    pass\n",
        "def create():\n    pass\n",
    ),
    (
        "ts-module",
        "c/module.ts",
        "export function create(): void {}\nexport function remove(): void {}\n",
        "export function create(): void {}\n",
    ),
    (
        "tool-config",
        "c/tool.toml",
        "requires = [\"x\"]\n\n[[tools]]\nid = \"a\"\nparser = \"p\"\n\n[[tools]]\nid = \"b\"\n",
        "requires = []\n\n[[tools]]\nid = \"a\"\n\n[[tools]]\nid = \"b\"\nparser = \"p\"\n",
    ),
    ("notes", "c/notes.txt", "notes\n", "other notes\n"),
];

/// The templates of `CONTRACTS`, each with a scenario of its file as rendered, placed by the workspace on files that
/// depart from them.
#[cfg(test)]
fn template_sources(directory: &std::path::Path) {
    let manifest = directory.join("tastemaker.toml");
    let text = std::fs::read_to_string(&manifest).expect("manifest");
    let entries = CONTRACTS
        .iter()
        .map(|(name, file, _, _)| format!("{{ template = \"p/{name}\", file = \"{file}\" }}"))
        .collect::<Vec<_>>()
        .join(", ");
    std::fs::write(manifest, format!("{text}templates = [{entries}]\n"))
        .expect("manifest with templates");
    for (name, file, content, written) in CONTRACTS {
        let quoted = |value: &str| toml::Value::String(value.to_owned()).to_string();
        // The file as written departs from its template: a mismatch where a language reads it, and `#unknown` at error for
        // the text file, which no language reads and which does not hold its template's text.
        let check = if name == "notes" {
            "unknown"
        } else {
            "template"
        };
        let found = CONTRACTED
            .iter()
            .filter(|rule| **rule == format!("p/{name}#{check}"))
            .map(|_| format!("{{ rule = \"{name}#{check}\", path = \"{file}\" }}"))
            .collect::<Vec<_>>()
            .join(", ");
        let failing = format!(
            "given.file = \"{file}\"\ngiven.files.\"{file}\" = {}\nthen.findings = [{found}]\n",
            quoted(written)
        );
        let definition = format!(
            "description = \"A file.\"\nwhy = \"A test needs a template.\"\ncontent = {}\n\n[[scenarios]]\nid = \"as-rendered\"\nscenario = \"The file as the template renders it.\"\ngiven.file = \"{file}\"\ngiven.files.\"{file}\" = {}\n\n[[scenarios]]\nid = \"as-written\"\nscenario = \"The file as the workspace writes it.\"\n{failing}",
            quoted(content),
            quoted(content),
        );
        let at = directory.join(format!("p/templates/{name}/0.1.0"));
        std::fs::create_dir_all(&at).expect("template directory");
        std::fs::write(at.join("template.toml"), definition).expect("template");
        let placed = directory.join(file);
        std::fs::create_dir_all(placed.parent().expect("a parent")).expect("directory");
        std::fs::write(placed, written).expect("placed file");
    }
}

/// Ordinary, documentation and nested block comments presented to the actual ignore reader.
#[cfg(test)]
fn comment_sources(directory: &std::path::Path) {
    let manifest = directory.join("tastemaker.toml");
    let text = std::fs::read_to_string(&manifest).expect("manifest");
    let text = text
        .lines()
        .filter(|line| !line.starts_with("specs ="))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(manifest, text).expect("rule-only manifest");
    let rule = directory.join("p/rules/marker/0.1.0/rule.toml");
    let function_rule = std::fs::read_to_string(&rule)
        .expect("rule")
        .replace(
            "from \"comment\" where text = \" MARKER\"",
            "from \"function\" where name = \"bad\"",
        )
        .replace(
            "given.files.\"src/notes.toml\" = \"# MARKER\\n\"",
            "given.files.\"src/lib.rs\" = \"fn bad() {}\\n\"",
        )
        .replace("path = \"src/notes.toml\"", "path = \"src/lib.rs\"")
        .replace(
            "given.files.\"src/notes.toml\" = \"# other\\n\"",
            "given.files.\"src/lib.rs\" = \"fn good() {}\\n\"",
        );
    std::fs::write(rule, function_rule).expect("function rule");
    for (name, source) in [
        (
            "ordinary",
            "// ignore: p/marker - ordinary directive\nfn bad() {}\n",
        ),
        (
            "inline",
            "fn bad() {} // ignore: p/marker - trailing directive\n",
        ),
        (
            "doc",
            "/// ignore: p/marker - a documentation comment of one line is a directive\nfn bad() {}\n",
        ),
        (
            "block",
            "/* ignore: p/marker - a block comment of one line is a directive */\nfn bad() {}\n",
        ),
        (
            "nested",
            "/* outer /* ignore: p/marker - nested */ tail */\nfn bad() {}\n",
        ),
    ] {
        std::fs::write(directory.join(format!("src/{name}.rs")), source).expect("source");
    }
}

/// The sources a command line's markers ask the workspace for: the comment contract's and the template contract's.
#[cfg(test)]
pub(crate) fn sources(directory: &std::path::Path, line: &[&str]) -> (bool, bool) {
    let comments = line.contains(&"comment-contract");
    if comments {
        comment_sources(directory);
    }
    let contracted = line.contains(&"template-contract");
    if contracted {
        template_sources(directory);
    }
    if line.contains(&"symbolic-link") {
        linked_source(directory);
    }
    (comments, contracted)
}

/// A symbolic link in the part `src/a`, which no template places, to the part's `errors.toml`, which holds the marker:
/// were the link followed, the marker would be found through it too.
#[cfg(test)]
fn linked_source(directory: &std::path::Path) {
    let link = directory.join("src/a/link.toml");
    #[cfg(unix)]
    std::os::unix::fs::symlink("errors.toml", &link).expect("the link");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file("errors.toml", &link).expect("the link");
}

/// Validates the fixture's package, which every row checks against: a package not proven fails the row.
#[cfg(test)]
pub(crate) fn proven(directory: &tempfile::TempDir) -> Result<(), crate::outcome::Outcome> {
    use tastemaker_core::inbound::use_cases::validate::interface::ValidateUseCase as _;
    let validation = crate::resources::workspace::validate::composition::build(
        &edge(directory).store,
        &crate::resources::workspace::validate::connections::connect(
            &edge(directory).store,
            &edge(directory).start,
            &edge(directory).build,
            tastemaker_core::domain::writing::types::Writing::Write,
        ),
    )
    .validate(
        tastemaker_core::inbound::use_cases::validate::types::ValidateRequest {
            start: edge(directory).start,
            only: Vec::new(),
        },
    )
    .map_err(|_error| crate::outcome::Outcome::Error)?;
    if validation
        .proved
        .iter()
        .any(|proved| proved.proof != tastemaker_core::domain::proofs::types::Proof::Proven)
    {
        return Err(crate::outcome::Outcome::Error);
    }
    Ok(())
}
