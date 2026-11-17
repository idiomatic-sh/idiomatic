//! What the `import_rows` service writes and keeps: the query of each built-in rule, and the row each relation keeps from
//! the cells a rule answered.

use super::service::{keep, query};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::types::CellText;
use crate::domain::comments::types::ByteSpan;
use crate::domain::findings::types::ByteOffset;
use crate::domain::imports::types::{
    Import, ImportColumn, ImportPlace, ImportRows, ModuleName, Name, NameBinding, Owner, Package,
    PackageBinding, Relation, ScopeKind, Target, UseScope, WrittenPath,
};
use crate::domain::paths::types::RepoPath;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The query of the rule over a relation.
    Query(Relation),
    /// The rows a relation keeps from one row of cells: `target`, `binding` and `package`, at the span 3 to 7.
    Kept(Relation),
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
            binding: NameBinding::Binds(ModuleName("other".into())),
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
        Relation::Scopes => drop(rows.scopes.insert(
            path("a.txt"),
            vec![UseScope {
                span: place(),
                kind: ScopeKind::Module,
            }],
        )),
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
        name: CaseName("the_rule_over_the_scopes_keys_a_row_by_its_node"),
        word: Word::Succeeds,
        input: || Question::Query(Relation::Scopes),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_scope_keeps_its_place_and_whether_it_is_a_module"),
        word: Word::Succeeds,
        input: || Question::Kept(Relation::Scopes),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_owner_keeps_its_package"),
        word: Word::Succeeds,
        input: || Question::Kept(Relation::Owners),
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
        Relation::Scopes => {
            r#"let uses = select id from "syntax-tree" where kind = "use" and text = "";
let blocks = select id, kind, file, span from "syntax-tree" where kind = "stmt-list" and text = "";
let modules = select id, kind, file, span from "syntax-tree" where kind = "item-list" and text = "";
let parents = select r.parent as id from uses as l join "syntax-child" as r on l.id = r.child;
let held = select r.id as id, r.kind as kind, r.file as file, r.span as span from parents as l join blocks as r on l.id = r.id;
let used = project(from=held, columns=["id", "kind", "file", "span"]);
let rows = union(left=used, right=modules);
emit rows subject "file" key("id");
"#
        }
    }
}

fn subject(question: Input) -> Value {
    match question {
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
                    "kind" => "item-list",
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

mod imports_read;
