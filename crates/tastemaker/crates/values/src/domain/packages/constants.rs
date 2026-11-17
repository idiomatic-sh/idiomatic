//! The names the `packages` concept writes and reads: directory names of a package and the files of a definition.

/// A package's rules, as its directory and a `tastemaker.toml`'s entry list name them.
pub const RULES: &str = "rules";
/// A package's specs, as its directory and a `tastemaker.toml`'s entry list name them.
pub const SPECS: &str = "specs";
/// A package's templates, as its directory and a `tastemaker.toml`'s entry list name them.
pub const TEMPLATES: &str = "templates";
/// A rule's definition file.
pub const RULE_FILE: &str = "rule.toml";
/// A spec's definition file.
pub const SPEC_FILE: &str = "spec.toml";
/// A template's definition file.
pub const TEMPLATE_FILE: &str = "template.toml";
/// A package's manifest.
pub const PACKAGE_FILE: &str = "package.toml";
/// What a definition kind reads as in a message and in the lock.
pub const RULE_WORD: &str = "rule";
/// What a spec reads as.
pub const SPEC_WORD: &str = "spec";
/// What a template reads as.
pub const TEMPLATE_WORD: &str = "template";
/// The version a definition `create` writes starts at.
pub const FIRST_VERSION: &str = "0.1.0";
/// A new rule: what it checks, why, its query and diagnostic, and the two cases every rule ships. Its scenarios'
/// files end in [`SCENARIO_EXTENSION`], which `create` writes as the extension the person names.
// ignore: idiomatic.arch/serialize-dont-format - the skeleton `create` writes is fixed example text for a person to edit, with comments, aligned columns and placeholders a serializer would drop; the written file is the person's to edit
pub const RULE_SKELETON: &str = r#"description = "What the rule checks, on one line."
why         = "Why it matters, and what a reader does about a finding."

query = '''
let empty = select * from "filesystem.line-count" where lines = 0;
let nodes = select l."id" as "id", l."file" as "file", l."span" as "span" from "syntax-tree" as l;
let children = select r."child" as "id" from "syntax-child" as r;
let roots = antijoin(left=nodes, right=children, left-on=["id"], right-on=["id"]);
let found = select r."file" as "file", r."span" as "span" from roots as r join empty as e on r."file" = e."path";
emit found subject "file" key("span");
'''

[diagnostic]
primary = "span"
message = "this file is empty"
help    = "What to change, and where; and whether an ignore with a reason is ever right."

[[scenarios]]
id       = "an-empty-file-is-reported"
scenario = "A file with nothing in it is reported at its first line."
given.include = ["src/**"]
given.files."src/one.<extension>" = ""
then.findings = [{ path = "src/one.<extension>", line = 1 }]

[[scenarios]]
id       = "a-file-with-text-is-not-reported"
scenario = "A file that holds something is not reported, and one the include leaves out is not read."
given.include = ["src/**"]
given.files."src/one.<extension>" = "1\n"
given.files."docs/one.<extension>" = ""
"#;
/// What the rule's and the template's skeleton scenario files end in, until `create` writes the extension the person names.
pub const SCENARIO_EXTENSION: &str = "<extension>";
/// A new spec.
// ignore: idiomatic.arch/serialize-dont-format - the skeleton `create` writes is fixed example text for a person to edit, with comments, aligned columns and placeholders a serializer would drop; the written file is the person's to edit
pub const SPEC_SKELETON: &str = r##"description = "What kind of directory the spec describes, on one line."
why         = "What the spec is for, and why that kind of directory has this shape."
closed      = false

templates = []
rules     = []

[[scenarios]]
id       = "directory-is-there"
scenario = "A directory the spec describes has no finding when it is there."
given.at = "demo"
given.files."demo/notes.txt" = "A file.\n"

[[scenarios]]
id       = "directory-is-missing"
scenario = "A directory the spec describes is reported when it is missing."
given.at = "demo"
given.files."other/notes.txt" = "A file.\n"
then.findings = [{ rule = "#count", path = "tastemaker.toml" }]
"##;
/// A new template, its content inline.
// ignore: idiomatic.arch/serialize-dont-format - the skeleton `create` writes is fixed example text for a person to edit, with comments, aligned columns and placeholders a serializer would drop; the written file is the person's to edit
pub const TEMPLATE_SKELETON: &str = r##"description = "What the file is, on one line."
why         = "What the file is for, and what someone needs before placing or filling it."
content     = ''

[[scenarios]]
id       = "file-holds-the-content"
scenario = "A placed file that holds the template's content has no finding."
given.file = "example.<extension>"
given.files."example.<extension>" = ""

[[scenarios]]
id       = "file-is-missing"
scenario = "A placed file that is missing is reported."
given.file = "example.<extension>"
then.findings = [{ rule = "#count", path = "tastemaker.toml" }]
"##;

/// How a lock that is not on disk is said.
pub const LOCK_MISSING: &str = "missing";

/// How a lock that differs from the one a load makes is said.
pub const LOCK_STALE: &str = "stale";

/// Each asset version's cumulative Markdown changelog.
pub const CHANGELOG_FILE: &str = "CHANGELOG.md";

/// A version heading in the package changelog.
pub const CHANGELOG_HEADING: &str = "## ";

/// The local workspace and asset package created by `init`: the package, the `unchecked` template copied into it, which
/// the workspace file `init` writes after them places every path the project already holds with, and the lock that pins
/// it, so the `tm check` `init` names next runs without `--fix`.
pub const INITIAL_FILES: [(&str, &str); 3] = [
    (
        "tastemaker/package.toml",
        r#"name = "local"
description = "Project-owned assets."
"#,
    ),
    (
        "tastemaker/templates/unchecked/0.1.0/template.toml",
        // ignore: idiomatic.arch/serialize-dont-format - the local package `init` writes is fixed text with comments a serializer would drop; the written file is the person's to edit
        r#"description = "A file placed as it is: Tastemaker accounts for it and checks none of its contents."
why = "A closed directory accounts for every file in it. A file or a whole directory whose contents are its owner's (notes, tooling, dotfiles) is placed by this template, so it is accounted for without a template per file; anything else in that directory stays a `#closed` finding."
content = ''

[[scenarios]]
id = 'any-file'
scenario = 'A file is placed whatever it holds.'
given.file = 'notes/{*path}'
given.count = { min = 1 }
given.files.'notes/plan.md' = '''
# plan
'''
given.files.'notes/tools/run.sh' = '''
#!/bin/sh
echo anything goes
'''

[[scenarios]]
id = 'file-missing'
scenario = 'A listed file that is not there is a count finding.'
given.file = 'LICENSE'
then.findings = [
  { rule = 'unchecked#count', path = 'tastemaker.toml' },
]
"#,
    ),
    (
        "tastemaker.lock",
        // ignore: idiomatic.arch/serialize-dont-format - the local package `init` writes is fixed text with comments a serializer would drop; the written file is the person's to edit
        r#"# Written by Tastemaker: the packages this workspace resolved,
# direct and transitive, and the definition releases it uses. Do not edit it by hand.
version = 1
dependencies = ["local"]

[packages.local]
path = "tastemaker"

[packages.local.templates]
unchecked = "0.1.0"
"#,
    ),
];
/// The version of Tastemaker that runs, which a root `tastemaker.toml` names in `tastemaker`.
pub const TASTEMAKER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The workspace file `init` writes, closed as every directory is unless it says otherwise: [`INIT_ENTRIES`] stands for one entry per path the project already holds.
pub const INIT_WORKSPACE: &str = concat!(
    "tastemaker = \"",
    env!("CARGO_PKG_VERSION"),
    // ignore: idiomatic.arch/serialize-dont-format - the workspace file `init` writes is fixed text with one placeholder for the project's entries and a description that quotes commands; the written file is the person's to edit
    r#""
workspace = { name = "local", dependencies = [{ package = "local", path = "tastemaker" }] }
templates = [
  { template = "local/unchecked", file = "tastemaker.lock", count = { min = 0 }, description = "The lock `tm check --fix` writes: the definition versions the workspace resolved." },
  { template = "local/unchecked", file = "asymptote.toml", count = { min = 0 }, description = "The tools `asymptote check` runs, as it is: unchecked until a spec or template places it." },
  { template = "local/unchecked", file = ".asymptote/{*asymptote}", count = { min = 0 }, description = "The lock `asymptote up` writes, as it is: unchecked until a spec or template places it." },
<entries>]
"#
);
/// Where [`INIT_WORKSPACE`] takes its entries.
pub const INIT_ENTRIES: &str = "<entries>";
/// The workspace file's path, relative to the project.
pub const INIT_WORKSPACE_FILE: &str = "tastemaker.toml";
/// One entry of [`INIT_WORKSPACE`] for a file at the project's top: `<path>` is its name.
pub const INIT_FILE_ENTRY: &str = "  { template = \"local/unchecked\", file = \"<path>\", description = \"`<path>`, as it is: unchecked until a spec or template places it.\" },\n";
/// One entry of [`INIT_WORKSPACE`] for a directory at the project's top: `<path>` is its name and `<capture>` a capture name
/// of its own.
pub const INIT_DIRECTORY_ENTRY: &str = "  { template = \"local/unchecked\", file = \"<path>/{*<capture>}\", description = \"`<path>/`, as it is: unchecked until a spec places it.\" },\n";
/// How a TOML basic string writes a control character: this, then its code point in four hexadecimal digits.
pub const INIT_UNICODE_ESCAPE: &str = "\\u";
/// Where an entry takes its path.
pub const INIT_PATH: &str = "<path>";
/// Where a directory's entry takes its capture name.
pub const INIT_CAPTURE: &str = "<capture>";

/// The key an entry names its definition's package by: in a `tastemaker.toml` entry, and as the column of the placed
/// relation that carries that entry's package.
pub const PACKAGE_KEY: &str = "package";
/// The capture name of a directory entry `init` writes when the directory's name holds no letter or digit.
pub const INIT_CAPTURE_FALLBACK: &str = "dir";
/// What joins the words of a capture name `init` makes.
pub const INIT_CAPTURE_JOIN: &str = "-";
