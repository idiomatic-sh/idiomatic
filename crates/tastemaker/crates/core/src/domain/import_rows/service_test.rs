//! What the `import_rows` service writes and keeps: the query of each built-in rule, and the row each relation keeps from
//! the cells a rule answered.

use super::service::{imports, keep, query};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::types::CellText;
use crate::domain::comments::types::ByteSpan;
use crate::domain::findings::types::ByteOffset;
use crate::domain::imports::types::{
    Import, ImportColumn, ImportPlace, ImportRows, Name, Owner, Package, PackageBinding, Relation,
    Target, WrittenPath,
};
use crate::domain::paths::types::RepoPath;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The query of the rule over a relation.
    Query(Relation),
    /// The rows a relation keeps from one row of cells: `target`, `binding` and `package`, at the span 3 to 7.
    Kept(Relation),
    /// What a name written with any separator reaches: the package its file's own package depends on by that binding.
    Resolved(&'static str),
    /// The rows the packages keep from a row whose `target` is empty: a package the repository does not hold.
    Outside,
}

type Input = Question;
/// What the service answered: a query's text, or nothing when the rows kept are the expected ones.
type Value = String;
type Failure = ();

fn path(text: &str) -> RepoPath {
    RepoPath(text.into())
}

fn place() -> ImportPlace {
    ImportPlace::At(ByteSpan {
        start: ByteOffset(3),
        end: ByteOffset(7),
    })
}

/// The rows a relation keeps from the one row of cells.
fn expected(relation: Relation) -> ImportRows {
    let mut rows = ImportRows::default();
    match relation {
        Relation::Edges => rows.edges.push(Import {
            source: path("a.txt"),
            span: place(),
            written: WrittenPath("crate::b".into()),
            target: Target::File(path("b.txt")),
        }),
        Relation::Names => rows.names.push(Name {
            source: path("a.txt"),
            written: WrittenPath("b.txt".into()),
            span: place(),
        }),
        Relation::Packages => rows.packages.push(Package {
            source: path("a.txt"),
            target: path("b.txt"),
            binding: PackageBinding("other".into()),
        }),
        Relation::Owners => rows.owners.push(Owner {
            path: path("a.txt"),
            package: path("c.toml"),
        }),
    }
    rows
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_rule_over_the_edges_keys_a_row_by_its_target_and_span"),
        word: Word::Succeeds,
        input: || Question::Query(Relation::Edges),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_rule_over_the_names_keys_a_row_by_its_occurrence"),
        word: Word::Succeeds,
        input: || Question::Query(Relation::Names),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_rule_over_the_packages_keys_a_row_by_its_target_binding_and_kind"),
        word: Word::Succeeds,
        input: || Question::Query(Relation::Packages),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_edge_keeps_its_target_its_place_and_its_path_as_written"),
        word: Word::Succeeds,
        input: || Question::Kept(Relation::Edges),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_name_keeps_its_written_path_and_place"),
        word: Word::Succeeds,
        input: || Question::Kept(Relation::Names),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_package_keeps_its_target_and_binding"),
        word: Word::Succeeds,
        input: || Question::Kept(Relation::Packages),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_package_the_repository_does_not_hold_keeps_no_row"),
        word: Word::Succeeds,
        input: || Question::Outside,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_rule_over_the_owners_keys_a_row_by_its_path"),
        word: Word::Succeeds,
        input: || Question::Query(Relation::Owners),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_owner_keeps_its_package"),
        word: Word::Succeeds,
        input: || Question::Kept(Relation::Owners),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_name_written_with_colons_reaches_the_package_its_package_depends_on"),
        word: Word::Succeeds,
        input: || Question::Resolved("my_lib::thing"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_name_written_with_dots_reaches_the_package_its_package_depends_on"),
        word: Word::Succeeds,
        input: || Question::Resolved("my_lib.thing"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "a_scoped_name_written_with_slashes_reaches_the_package_by_its_whole_binding",
        ),
        word: Word::Succeeds,
        input: || Question::Resolved("@scope/my-lib/thing"),
        expect: Expect::Value(String::new()),
    },
];

/// The query each relation's rule is written as.
fn written(relation: Relation) -> &'static str {
    match relation {
        Relation::Edges => {
            r#"let nodes = select id, kind, file, span from "syntax-tree" where text = "";
let paths = select * from nodes where kind = "path";
let visibility = select * from nodes where kind = "visibility";
let nested = axis(from=visibility, relation="syntax-child", direction=descendant, domain=paths);
let scopes = select r.file as source, r.span as span from nested as l join paths as r on l.related = r.id;
let written = select * from "source-dependency";
let rows = antijoin(left=written, right=scopes, left-on=["source", "span"], right-on=["source", "span"]);
emit rows subject "source" key("target", "span");
"#
        }
        Relation::Names => {
            "let rows = select * from \"static-import\";\nemit rows subject \"file\" key(\"occurrence\");\n"
        }
        Relation::Packages => {
            "let rows = select * from \"package-dependency\";\nemit rows subject \"source\" key(\"target\", \"binding\", \"kind\");\n"
        }
        Relation::Owners => {
            "let rows = select * from \"file-package\";\nemit rows subject \"path\" key(\"path\");\n"
        }
    }
}

/// What the name `written` in `src/a.x` reaches, when its package `app.m` depends on `my-lib` and `@scope/my-lib`.
fn resolved(written: &'static str) -> String {
    let rows = ImportRows {
        edges: Vec::new(),
        names: vec![Name {
            source: path("src/a.x"),
            written: WrittenPath(written.into()),
            span: place(),
        }],
        packages: vec![
            Package {
                source: path("app.m"),
                target: path("lib/lib.m"),
                binding: PackageBinding("my-lib".into()),
            },
            Package {
                source: path("app.m"),
                target: path("scoped/lib.m"),
                binding: PackageBinding("@scope/my-lib".into()),
            },
        ],
        owners: vec![Owner {
            path: path("src/a.x"),
            package: path("app.m"),
        }],
    };
    let wanted = if written.starts_with('@') {
        "scoped/lib.m"
    } else {
        "lib/lib.m"
    };
    match imports(&rows).first().map(|import| &import.target) {
        Some(Target::File(target)) if &*target.0 == wanted => String::new(),
        other => format!("{other:?}"),
    }
}

fn subject(question: Input) -> Value {
    match question {
        Question::Resolved(written) => resolved(written),
        Question::Outside => {
            let mut rows = ImportRows::default();
            let column = |name: ImportColumn<'_>| match name.0 {
                "target" => Some(CellText(String::new().into())),
                "binding" => Some(CellText("outside".into())),
                _ => None,
            };
            keep(
                &mut rows,
                Relation::Packages,
                path("a.toml"),
                &RepoPath::default(),
                &column,
                ImportPlace::Unplaced,
            );
            if rows == ImportRows::default() {
                String::new()
            } else {
                format!("{rows:?}")
            }
        }
        Question::Query(relation) => {
            let text = query(relation);
            if &*text.0 == written(relation) {
                String::new()
            } else {
                text.0.into_string()
            }
        }
        Question::Kept(relation) => {
            let mut rows = ImportRows::default();
            let column = |name: ImportColumn<'_>| {
                let cell = match name.0 {
                    "target" => "b.txt",
                    "binding" => "other",
                    "package" => "c.toml",
                    "written" => "crate::b",
                    _ => return None,
                };
                Some(CellText(cell.into()))
            };
            keep(
                &mut rows,
                relation,
                path("a.txt"),
                &RepoPath::default(),
                &column,
                place(),
            );
            if rows == expected(relation) {
                String::new()
            } else {
                format!("{rows:?}")
            }
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
