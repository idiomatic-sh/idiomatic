//! What the `import_rows` service writes and keeps: the query of each built-in rule, and the row each relation keeps from
//! the cells a rule answered.

use std::collections::BTreeMap;

use super::service::{imports, keep, query};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::types::CellText;
use crate::domain::comments::types::ByteSpan;
use crate::domain::findings::types::ByteOffset;
use crate::domain::imports::types::{
    Import, ImportColumn, ImportPlace, ImportRows, ModuleName, Name, Owner, Package,
    PackageBinding, Relation, Scope, ScopeKind, Target, WrittenPath,
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
    /// What the imports of `src/a.x` written `use crate::contract` (binding `contract`, at 0 to 21) are, with an edge to
    /// `src/contract/collect.x` written at a use site, at 40 to 56, as this path, in these scopes.
    UseSite(&'static str, Layout),
    /// What a `static-import` row with a `target` and no `binding` keeps, and what `src/a.x` imports when that row is
    /// its one import (at 0 to 21) and a use-site path `x::Y` (at 40 to 56) reaches `src/x.x`.
    Unbound,
}

/// The blocks and inline modules around the `use` (0 to 21) and the use-site path (40 to 56).
#[derive(Clone, Copy, Debug)]
enum Layout {
    /// None: both are at the top of the file.
    File,
    /// An inline module (30 to 60) holds the path alone.
    ModuleAroundPath,
    /// A block (0 to 25) holds the `use` alone, as in one function while the path is in another.
    BlockAroundUse,
    /// An inline module (0 to 25) holds the `use` alone, as a `#[cfg(test)] mod tests` does.
    ModuleAroundUse,
    /// A block (0 to 60) holds both.
    BlockAroundBoth,
    /// An inline module (0 to 60) holds both.
    ModuleAroundBoth,
    /// An inline module (0 to 70) holds both, and another (22 to 30) nested in it ends before the path.
    ModuleBesidePath,
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
            binding: Some(ModuleName("other".into())),
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
            vec![Scope {
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
    Case {
        name: CaseName("a_path_at_a_use_site_through_a_name_an_import_binds_is_that_imports"),
        word: Word::Succeeds,
        input: || Question::UseSite("contract::Collect", Layout::File),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_at_a_use_site_from_the_package_root_is_an_import_of_its_own"),
        word: Word::Succeeds,
        input: || Question::UseSite("crate::contract::Collect", Layout::File),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_in_an_inline_module_sees_no_import_written_outside_it"),
        word: Word::Succeeds,
        input: || Question::UseSite("contract::Collect", Layout::ModuleAroundPath),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_outside_the_block_of_an_import_sees_no_binding_of_it"),
        word: Word::Succeeds,
        input: || Question::UseSite("contract::Collect", Layout::BlockAroundUse),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_outside_the_inline_module_of_an_import_sees_no_binding_of_it"),
        word: Word::Succeeds,
        input: || Question::UseSite("contract::Collect", Layout::ModuleAroundUse),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_in_the_inline_module_of_an_import_is_that_imports"),
        word: Word::Succeeds,
        input: || Question::UseSite("contract::Collect", Layout::ModuleAroundBoth),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_inline_module_that_ends_before_a_path_does_not_shut_it_from_an_import"),
        word: Word::Succeeds,
        input: || Question::UseSite("contract::Collect", Layout::ModuleBesidePath),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_import_that_binds_no_name_is_kept_and_a_path_is_never_its"),
        word: Word::Succeeds,
        input: || Question::Unbound,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_in_the_block_of_an_import_is_that_imports"),
        word: Word::Succeeds,
        input: || Question::UseSite("contract::Collect", Layout::BlockAroundBoth),
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

/// What the name `written` in `src/a.x` reaches, when its package `app.m` depends on `my-lib` and `@scope/my-lib`.
fn resolved(written: &'static str) -> String {
    let rows = ImportRows {
        edges: Vec::new(),
        names: vec![Name {
            source: path("src/a.x"),
            written: WrittenPath(written.into()),
            binding: Some(ModuleName("thing".into())),
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
        scopes: BTreeMap::new(),
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

fn at(start: usize, end: usize) -> ImportPlace {
    ImportPlace::At(ByteSpan {
        start: ByteOffset(start),
        end: ByteOffset(end),
    })
}

/// The imports read when the use-site path is `written`: the `use` alone when the path starts at the name it binds,
/// the `use` and the path otherwise.
fn use_site(written: &'static str, layout: Layout) -> String {
    let scope = |start, end, kind| Scope {
        span: at(start, end),
        kind,
    };
    let (scopes, through) = match layout {
        Layout::File => (Vec::new(), true),
        Layout::ModuleAroundPath => (vec![scope(30, 60, ScopeKind::Module)], false),
        Layout::BlockAroundUse => (vec![scope(0, 25, ScopeKind::Block)], false),
        Layout::ModuleAroundUse => (vec![scope(0, 25, ScopeKind::Module)], false),
        Layout::BlockAroundBoth => (vec![scope(0, 60, ScopeKind::Block)], true),
        Layout::ModuleAroundBoth => (vec![scope(0, 60, ScopeKind::Module)], true),
        Layout::ModuleBesidePath => (
            vec![
                scope(0, 70, ScopeKind::Module),
                scope(22, 30, ScopeKind::Module),
            ],
            true,
        ),
    };
    let declared = Import {
        source: path("src/a.x"),
        span: at(0, 21),
        written: WrittenPath("crate::contract".into()),
        target: Target::File(path("src/contract.x")),
    };
    let site = Import {
        source: path("src/a.x"),
        span: at(40, 56),
        written: WrittenPath(written.into()),
        target: Target::File(path("src/contract/collect.x")),
    };
    let rows = ImportRows {
        edges: vec![declared.clone(), site.clone()],
        names: vec![Name {
            source: path("src/a.x"),
            written: WrittenPath("crate::contract".into()),
            binding: Some(ModuleName("contract".into())),
            span: at(0, 21),
        }],
        packages: Vec::new(),
        owners: Vec::new(),
        scopes: BTreeMap::from([(path("src/a.x"), scopes)]),
    };
    let wanted = if through && written.starts_with("contract") {
        vec![declared]
    } else {
        vec![declared, site]
    };
    let found = imports(&rows);
    if found == wanted {
        String::new()
    } else {
        format!("{found:?}")
    }
}

/// The row a `static-import` row without a `binding` keeps, and the imports of a file whose one import binds no name.
fn unbound() -> String {
    let mut rows = ImportRows::default();
    let column = |name: ImportColumn<'_>| match name.0 {
        "target" => Some(CellText("x".into())),
        _ => None,
    };
    keep(
        &mut rows,
        Relation::Names,
        path("src/a.x"),
        &RepoPath::default(),
        &column,
        at(0, 21),
    );
    let name = Name {
        source: path("src/a.x"),
        written: WrittenPath("x".into()),
        binding: None,
        span: at(0, 21),
    };
    if rows.names != vec![name] {
        return format!("{:?}", rows.names);
    }
    let site = Import {
        source: path("src/a.x"),
        span: at(40, 56),
        written: WrittenPath("x::Y".into()),
        target: Target::File(path("src/x.x")),
    };
    rows.edges.push(site.clone());
    let declared = Import {
        source: path("src/a.x"),
        span: at(0, 21),
        written: WrittenPath("x".into()),
        target: Target::Module,
    };
    let found = imports(&rows);
    if found == vec![declared, site] {
        String::new()
    } else {
        format!("{found:?}")
    }
}

fn subject(question: Input) -> Value {
    match question {
        Question::Unbound => unbound(),
        Question::UseSite(written, layout) => use_site(written, layout),
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
