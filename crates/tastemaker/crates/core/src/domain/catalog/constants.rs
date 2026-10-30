//! The words of the `catalog` concept: the headings and the fixed words its sections say.

/// The heading of a relation's columns.
pub const COLUMNS: &str = "Columns";

/// The heading of the languages that read a relation.
pub const LANGUAGES: &str = "Languages";

/// The heading of an operator's fields.
pub const FIELDS: &str = "Fields";

/// The heading of an operator's choices.
pub const OPTIONS: &str = "Options";

/// The heading of an operator's example.
pub const EXAMPLE: &str = "Example";

/// The heading of the grammar.
pub const RYOQL: &str = "RyoQL";

/// The heading of the walkthrough `explain ryoql` opens with, before the grammar.
pub const WRITING: &str = "Writing a query";

/// The heading of the worked queries after the walkthrough.
pub const WORKED: &str = "Worked queries";

/// How a query is written, a step a line: pick a relation, narrow it, join, emit.
pub const WALKTHROUGH: &[&str] = &[
    "1. Pick a relation: `let found = select * from \"comment\";` reads every row of `comment`. `tm list relations` lists the relations; `tm explain relation/<name>` shows one's columns and the languages that read it.",
    "2. Narrow it: `where` keeps the rows a filter holds for, as in `select * from \"comment\" where glob_any(text, \"*TODO*\")`. Filters are joined with `and` and `or`; `param max-lines: integer;` takes a value from the entry, read as `$max-lines`.",
    "3. Join: `select c.\"file\" as \"file\" from classes as c join services as p on c.\"file\" = p.\"path\"` pairs the rows of two steps on equal columns; `antijoin` keeps the rows of one that the other lacks. `tm list operators` lists every operator; `tm explain operator/<name>` shows one.",
    "4. Emit: `emit found subject \"file\" key(\"span\");` turns each row of a step into a finding, at the file and the span it names. A query has exactly one `emit`.",
];

/// Whole queries, each what it checks and its text: every one validates (the analysis adapter's test).
pub const WORKED_QUERIES: &[(&str, &str)] = &[
    (
        "No comment holds `TODO`:",
        "let found = select * from \"comment\" where glob_any(text, \"*TODO*\");\nemit found subject \"file\" key(\"span\");",
    ),
    (
        "No file imports what the entry forbids, its `params = { forbidden = [\"*.routers.*\"] }`:",
        "param forbidden: set<string>;\nlet found = select * from \"static-import\" where glob_any(target, $forbidden);\nemit found subject \"file\" key(\"span\");",
    ),
    (
        "Each `src/services/<pkg>/service.<extension>` declares a class named for its package, `todo_lists` as `TodoListsService`:",
        "let services = capture(from=\"filesystem.file\", path=\"path\", pattern=\"src/services/{pkg}/service.{extension}\", captures=[\"pkg\", \"extension\"]);\nlet classes = select * from \"defines\" where kind = \"class\";\nlet named = select c.\"file\" as \"file\", p.\"pkg\" as \"pkg\" from classes as c join services as p on c.\"file\" = p.\"path\" where c.name = concat(pascal(p.pkg), \"Service\");\nlet missing = antijoin(left=services, right=named, left-on=[\"path\"], right-on=[\"file\"]);\nemit missing subject \"path\" key(\"path\");",
    ),
];

/// What a language that reads a relation says.
pub const READS: &str = "reads";

/// What an operator's field that must be given says.
pub const REQUIRED: &str = crate::domain::topics::constants::REQUIRED;

/// What an operator's field that may be left out says.
pub const OPTIONAL: &str = crate::domain::topics::constants::OPTIONAL;

/// What separates the names of a list.
pub const SEPARATOR: &str = ", ";
