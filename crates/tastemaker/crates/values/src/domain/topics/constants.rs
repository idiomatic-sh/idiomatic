//! The topics `explain` answers, each in the words of the design (crates/tastemaker/docs).

use super::types::{Form, KeyTable, Topic, TopicBody, TopicName, TopicText};
use crate::domain::packages::constants::{
    RULE_SKELETON, RULE_WORD, SPEC_SKELETON, SPEC_WORD, TEMPLATE_SKELETON, TEMPLATE_WORD,
};

/// Every topic, in the order `explain --all` writes them.
pub const TOPICS: &[Topic] = &[
    Topic {
        name: TopicName("selectors"),
        body: TopicBody::Text(TopicText(
            "# Selectors\n\nWherever an entry says where, it writes a selector list: `include`, `exclude`, `rules-exclude` and `members`.\n\n| selector | written | selects |\n|---|---|---|\n| glob | `\"docs/**/*.md\"` | paths matching the pattern beneath the place; `*` one segment, `**` any depth, `?` one character, `{name}` a capture, `{*name}` whole segments |\n| regex | `{ regex = '^docs/.*\\.md$' }` | paths beneath the place the expression matches |\n| part | `{ part = \"use-case\" }` | the directories of the instances of a spec directly beneath the place |\n| recursive part | `{ part = \"use-case\", recursive = true }` | the same at any depth |\n| placed-by | `{ placed-by = \"errors\" }` | the files one template entry placed beneath the place |\n\nA selector covers what it selects and everything beneath it. `exclude` subtracts. A `{ variable = \"x\" }` in a list splices the variable's list in. A `{name:dir}` is refused in a selector list: it is written only in a template entry's `file`.\n\nA `\\` makes the character after it stand for itself: `\"data\\\\*.txt\"` is the file named `data*.txt`, never a wildcard, and `\\{`, `\\}`, `\\?` and `\\\\` the same (in a TOML basic string the backslash is written twice; a literal string, `'data\\*.txt'`, writes it once). `tm init` writes each name it finds this way.\n",
        )),
    },
    Topic {
        name: TopicName("variables"),
        body: TopicBody::Text(TopicText(
            "# Variables\n\nA `tastemaker.toml` or a spec names a value once and refers to it anywhere a value or a selector list is written:\n\n```toml\nvariables = { tests = [\"**/tests/**\", \"**/fixtures/**\"] }\nrules = [{ rule = \"p/r\", include = [\"**\"], exclude = [{ variable = \"tests\" }] }]\n```\n\nA variable is scoped to the file that declares it. In a selector list a list variable is spliced in; anywhere else it is the whole value. A reference to no variable, and a variable no entry refers to, are `tastemaker#format` errors.\n",
        )),
    },
    Topic {
        name: TopicName("scenarios"),
        body: TopicBody::Text(TopicText(
            "# Scenarios\n\nEvery rule, spec and template proves itself with `[[scenarios]]` in its own file, each one a Gherkin Scenario: it writes out every file it needs and depends on nothing but itself. A `scenario` line says what it shows, `given` is the project, `then` is what is expected:\n\n```toml\n[[scenarios]]\nid       = \"fixme-in-a-note\"\nscenario = \"A FIXME in a note is reported.\"\ngiven.include = [\"**\"]\ngiven.files.\"docs/notes.md\" = \"FIXME\\n\"\nthen.findings = [{ path = \"docs/notes.md\", line = 1 }]\n```\n\n| Gherkin | field |\n|---|---|\n| Scenario | `id` and `scenario` |\n| Given | `given.files`, `given.include`, `given.params` |\n| Then | `then.findings` |\n\nAn `id` is kebab-case, unique in the asset and says what the scenario covers; `pass` and `fail` are no id. A scenario that lists no `then.findings` expects no finding; one that lists them expects those findings. Every asset has at least one scenario that passes and one that fails. `tm validate` writes each scenario's files to a project of its own, held in memory, and checks them there as any project. A rule's scenario names `given.include`; a spec's names `given.at`, the instance's directory, and its findings name their rule; a template's names `given.file`. A scenario that brings a `tastemaker.toml` in `given.files` is placed by it, not by `given.include`, `given.at` or `given.file`. That file is a root: it declares `workspace` and the packages it depends on, each `path` read from the directory of the asset's own file (`../../..` from `rules/<name>/<version>/` is the package itself), and names each definition `package/name`, never with a version:\n\n```toml\n[[scenarios]]\nid       = \"placed-by-its-own-workspace\"\nscenario = \"A scenario that brings its own tastemaker.toml is placed by it.\"\ngiven.files.\"tastemaker.toml\" = '''\nworkspace = { name = \"scenario\", dependencies = [{ package = \"local\", path = \"../../..\" }] }\nrules = [{ rule = \"local/fixme\", include = [\"docs/**\"] }]\n'''\ngiven.files.\"docs/notes.md\" = \"FIXME\\n\"\nthen.findings = [{ path = \"docs/notes.md\", line = 1 }]\n```\n\nWhat every scenario of a definition starts from is written once, in the definition's own `[given]`, and each scenario writes only what differs: a key it writes wins, and its files and params are added to the shared ones, a path or a name it writes again taking its own text or value:\n\n```toml\n[given]\ninclude = [\"**\"]\nfiles.\"docs/index.md\" = \"# Index\\n\"\n\n[[scenarios]]\nid       = \"fixme-in-a-note\"\nscenario = \"A FIXME in a note is reported.\"\ngiven.files.\"docs/notes.md\" = \"FIXME\\n\"\nthen.findings = [{ path = \"docs/notes.md\", line = 1 }]\n\n[[scenarios]]\nid       = \"plain-note\"\nscenario = \"A note with no FIXME is not reported.\"\ngiven.files.\"docs/notes.md\" = \"done\\n\"\n```\n\nA version whose files are unchanged is proved once: its proof is kept under the digest of its files and the builds.\n",
        )),
    },
    Topic {
        name: TopicName("ignores"),
        body: TopicBody::Text(TopicText(
            "# Ignores\n\nA finding that is accepted is ignored where it is, in the file's line comment, naming the rule and saying why, in the file's own comment syntax:\n\n```text\n// ignore: idiomatic.arch/file-length - the table is one list read whole     (a language that comments with //)\n# ignore: idiomatic.arch/file-length - the table is one list read whole      (a language that comments with #)\n```\n\nAt the top of a file, before any code, an ignore covers every finding of its rule in the file; at the end of a line, that line; on its own line after code, the item on the next line of code. An ignore with no rule, no reason, an unknown rule, or that covers nothing, is a `tastemaker#ignore` error.\n",
        )),
    },
    Topic {
        name: TopicName("store"),
        body: TopicBody::Text(TopicText(
            "# The store\n\nOne store at the root of the project, `.tastemaker_cache/`, gitignored, with ryo's cache inside it at `.tastemaker_cache/ryo`. The store is two files, each written whole by a run that succeeds and changes it: `.tastemaker_cache/validate.store`, the build that wrote it and the versions `validate` proved held; and `.tastemaker_cache/check.store`, the build that wrote it, the files `check`'s answer rests on, its findings and the rules it ran. A file another build wrote opens empty. Each worktree has its own project root, so its own store.\n",
        )),
    },
    Topic {
        name: TopicName(RULE_WORD),
        body: TopicBody::Form(Form {
            file: TopicText("rule.toml"),
            skeleton: TopicText(RULE_SKELETON),
            tables: RULE_TABLES,
            notes: &[],
        }),
    },
    Topic {
        name: TopicName(SPEC_WORD),
        body: TopicBody::Form(Form {
            file: TopicText("spec.toml"),
            skeleton: TopicText(SPEC_SKELETON),
            tables: SPEC_TABLES,
            notes: &[],
        }),
    },
    Topic {
        name: TopicName(TEMPLATE_WORD),
        body: TopicBody::Form(Form {
            file: TopicText("template.toml"),
            skeleton: TopicText(TEMPLATE_SKELETON),
            tables: TEMPLATE_TABLES,
            notes: TEMPLATE_NOTES,
        }),
    },
];
/// The index `explain --all` writes beside the topics and definitions, for a program to read first.
pub const INDEX: &str = "llms.txt";

/// The tables of a `rule.toml`; `[params.<name>]` and `[[scenarios]]` are the same in a spec and a template. One line per key:
/// `name | shape | required or optional | what it is`.
pub const RULE_TABLES: &[KeyTable] = &[
    KeyTable {
        title: TopicText("rule.toml"),
        keys: TopicText(
            "description | string | required | what the rule checks, on one line
why | string | required | why it matters, and what a reader does about a finding
params | table | optional | one `[params.<name>]` per value an entry passes
exceptions | table | optional | one `[exceptions.<name>]` per set of values the query is handed as `$exceptions`
query | string | required | RyoQL ending in one `emit`: each row is a finding (`explain ryoql`)
diagnostic | table | required | `[diagnostic]`: how a finding is said
scenarios | array of tables | optional | `[[scenarios]]`: the projects the rule is proved on, one that passes and one that fails at least (`explain scenarios`)
given | table | optional | `[given]`: what every scenario starts from; a scenario's own `given` adds to it, a key, file or param it writes again taking its own (`explain scenarios`)",
        ),
    },
    KeyTable {
        title: TopicText("[diagnostic]"),
        keys: TopicText(
            "primary | string | required | the column of the emitted row the finding is placed at
message | string | required | what is wrong
help | string | required | what to change, and whether an ignore with a reason is ever right",
        ),
    },
    KeyTable {
        title: TopicText("[params.<name>]"),
        keys: TopicText(
            "type | string or table | required | `string`, `identifier`, `integer`, `boolean`, `list<…>`, `set<…>` or `{ one-of = [\"a\", \"b\"] }`
description | string | optional | what the value means
from | string | optional | `directory-name`: the value defaults to the name of the instance's directory; `workspace-name`: to the workspace's name",
        ),
    },
    KeyTable {
        title: TopicText("[exceptions.<name>]"),
        keys: TopicText(
            "admits | array | required | the values admitted
why | string | required | why they are admitted",
        ),
    },
    KeyTable {
        title: TopicText("[[scenarios]]"),
        keys: TopicText(
            "id | string | required | the scenario's name: kebab-case, unique in the asset, saying what it covers
scenario | string | required | one sentence saying what it shows, as a Gherkin Scenario line
given | table | optional | `given.…`: the project, and where the asset is placed in it
then | table | optional | `then.…`: what is expected; none when nothing is found",
        ),
    },
    KeyTable {
        title: TopicText("[scenarios.given]"),
        keys: TopicText(
            "at | string | optional | a spec's scenario: the instance's directory, written as an entry's `include` is, with `{name}` captures
file | string | optional | a template's scenario: the file it places the template at, required unless `files` brings a `tastemaker.toml`
count | integer or table | optional | a template's scenario: how many files it places, as in an entry
include | selector list | optional | a rule's scenario: the files it reads (`explain selectors`)
exclude | selector list | optional | a rule's scenario: what it leaves out of `include`
params | table | optional | the values the scenario passes
files | table | optional | `given.files.\"<path>\" = \"<text>\"`: every file the scenario needs, written out; a `tastemaker.toml` places the asset itself, a root with its `workspace` and dependencies (`explain scenarios`)",
        ),
    },
    KeyTable {
        title: TopicText("[scenarios.then]"),
        keys: TopicText(
            "only | array | optional | the rules whose findings are compared, every rule's when left out
findings | array of tables | optional | `{ rule, path, line }` of each finding expected; none, or an empty list, when nothing is found",
        ),
    },
    KeyTable {
        title: TopicText("findings"),
        keys: TopicText(
            "rule | string | optional | the rule that finds it, required in a spec's or template's scenario; left out in a rule's scenario, it is the rule itself
path | string | required | the file
line | integer | optional | the line; the whole file when left out",
        ),
    },
];

/// The tables of a `spec.toml`.
pub const SPEC_TABLES: &[KeyTable] = &[
    KeyTable {
        title: TopicText("spec.toml"),
        keys: TopicText(
            "description | string | required | what kind of directory it describes, on one line
why | string | required | why that kind of directory has this shape
params | table | optional | `[params.<name>]`, as in `explain rule`
scenarios | array of tables | optional | `[[scenarios]]`, as in `explain rule`, each naming `given.at`
given | table | optional | `[given]`: what every scenario starts from; a scenario's own `given` adds to it, a key, file or param it writes again taking its own (`explain scenarios`)
closed | boolean | optional | `false` opens the directory; left out, it is closed and a file no template entry places is a `#closed` finding (`closed = true`, the default written out, is refused)
rules | array of tables | optional | rule entries: the rules an instance is read by
specs | array of tables | optional | spec entries: the directories an instance holds
templates | array of tables | optional | template entries: the files an instance holds
conditions | table | optional | `[conditions.<name>]`: a yes or no a `count` may depend on
variables | table | optional | values named once and referred to (`explain variables`)",
        ),
    },
    KeyTable {
        title: TopicText("entry of `rules`, `specs`, `templates`"),
        keys: TopicText(
            // ignore: idiomatic.arch/serialize-dont-format - a documentation table of a topic's keys (`name | type | note`), shown as help, never parsed or written as a document
            "alias | string | optional | the entry's own name, when a definition is placed twice
description | string | optional | what the entry is here
rule | string | optional | a rule entry's rule: a bare `name` the spec owns, or another package's `package/name@version`
spec | string | optional | a spec entry's spec, written as `rule` is
template | string | optional | a template entry's template, written as `rule` is
include | selector list | optional | where it applies (`explain selectors`); required on rule and spec entries
exclude | selector list | optional | what it leaves out
file | string | optional | a template entry's file, required on it; `{name}` captures one segment, `{*name}` the rest, `{name:dir}` one segment naming a directory beside the file, written only in the last segment
params | table | optional | the values passed; `{ param = \"x\" }` passes the spec's own, `{ variable = \"x\" }` a variable
count | integer or table | optional | on spec and template entries, refused on a rule entry, written only where it differs from what the path says (a literal path is one, a glob or capture at least one): `n`, `{ min = n }` or `{ when = { condition = \"c\" }, then = n, else = n }`
imports | table | optional | `{ default = \"deny\", allow = [\"…\"] }`: the modules the placed files may import",
        ),
    },
    KeyTable {
        title: TopicText("[conditions.<name>]"),
        keys: TopicText(
            "description | string | optional | the question it answers
entry | table | optional | the rule entry whose finding answers yes; the condition names it by `entry` or by `rule`, one of the two is required",
        ),
    },
    KeyTable {
        title: TopicText("imports"),
        keys: TopicText(
            "default | string | optional | `deny`: only what `allow` names may be imported
allow | array | optional | the module patterns allowed",
        ),
    },
];

/// The tables of a `template.toml`.
pub const TEMPLATE_TABLES: &[KeyTable] = &[KeyTable {
    title: TopicText("template.toml"),
    keys: TopicText(
        "description | string | required | what the file is, on one line
why | string | required | what the file is for
params | table | optional | `[params.<name>]`, as in `explain rule`
match | string | optional | `\"declarations\"`: the file declares what the content declares and nothing more, its bodies free, and a `tastemaker.toml` placed by a `declarations` template declares what the template writes and nothing more, except that each of its `variables` may hold values beyond the template's; `\"text\"`: the file's text is the content's, byte for byte; `\"contains\"` (the default, not written): the file holds the content's declarations and may hold more
content | string | required | the file's text, in Jinja
scenarios | array of tables | optional | `[[scenarios]]`, as in `explain rule`, each naming `given.file`
given | table | optional | `[given]`: what every scenario starts from; a scenario's own `given` adds to it, a key, file or param it writes again taking its own (`explain scenarios`)",
    ),
}];

/// What a template's content may read and call, beside Jinja's own.
pub const TEMPLATE_NOTES: &[TopicText] = &[
    TopicText(
        "- in scope: each param, each capture of the entry's `file`, `scaffold` (set only while a file is first written)",
    ),
    TopicText(
        "- `captures.<name>`, `present.<entry>`, `instances.<spec>` (each with `name` and `path`)",
    ),
    TopicText("- filters: `pascal`, `camel`, `snake`, `kebab`, `shouty`"),
    TopicText("- `refuse(\"why\")`: render nothing and say why"),
];

/// The topic the RyoQL grammar is, which ryo answers.
pub const RYOQL: &str = "ryoql";

/// The kind word `explain relation/<name>` and `list relations` name a relation of ryo's by.
pub const RELATION_WORD: &str = "relation";

/// The kind word `explain operator/<name>` and `list operators` name an operator of ryo's by.
pub const OPERATOR_WORD: &str = "operator";

/// How each kind of asset is written, one step a line: what `explain` says when asked nothing, and `explain --help`.
pub const AUTHORING: &[TopicText] = &[
    TopicText("rule:"),
    TopicText("  1. tm explain rule"),
    TopicText("  2. tm create rule/<name> --set extension=<ext>"),
    TopicText("  3. write a failing and a passing [[scenarios]]: tm explain scenarios"),
    TopicText("  4. write the query: tm explain ryoql, tm list relations, tm list operators"),
    TopicText("  5. tm validate rule/<name>, until it passes"),
    TopicText("  6. tm add <name> --at <dir>"),
    TopicText("  7. tm check"),
    TopicText("spec:"),
    TopicText("  1. tm explain spec"),
    TopicText("  2. tm create spec/<name>"),
    TopicText("  3. write [[scenarios]], each at an instance: tm explain scenarios"),
    TopicText("  4. write the entries: tm explain selectors, tm list"),
    TopicText("  5. tm validate spec/<name>, until it passes"),
    TopicText("  6. tm add <name> --at <dir>, then tm scaffold <name> --at <dir>"),
    TopicText("  7. tm check"),
    TopicText("template:"),
    TopicText("  1. tm explain template"),
    TopicText("  2. tm create template/<name> --set extension=<ext>"),
    TopicText("  3. write the content and its [[scenarios]]: tm explain scenarios"),
    TopicText("  4. tm validate template/<name>, until it passes"),
    TopicText("  5. tm add <name> --at <file>"),
    TopicText("  6. tm check"),
];

/// What separates the four parts of a key's line.
pub const PART: char = '|';

/// A key the reader refuses a table without.
pub const REQUIRED: &str = "required";

/// A key the reader reads a table without.
pub const OPTIONAL: &str = "optional";

/// The heading before the notes a form topic says after its tables.
pub const CONTENT_HEADING: &str = "\n## content\n\n";
