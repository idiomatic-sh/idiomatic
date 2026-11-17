//! What the imports of a file are, read from the rows the import relations kept: the name a use site writes, the
//! scopes around a `use`, and an import that binds no name.

use std::collections::BTreeMap;

use super::super::{path, place};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::types::CellText;
use crate::domain::comments::types::ByteSpan;
use crate::domain::findings::types::ByteOffset;
use crate::domain::import_rows::service::{imports, keep};
use crate::domain::imports::types::{
    Import, ImportColumn, ImportPlace, ImportRows, ModuleName, Name, NameBinding, Owner, Package,
    PackageBinding, Relation, ScopeKind, Target, UseScope, WrittenPath,
};
use crate::domain::paths::types::RepoPath;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// What a name written with any separator reaches: the package its file's own package depends on by that binding.
    Resolved(&'static str),
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
/// What the service answered, or nothing when it is the expected answer.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
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

/// What the name `written` in `src/a.x` reaches, when its package `app.m` depends on `my-lib` and `@scope/my-lib`.
fn resolved(written: &'static str) -> String {
    let rows = ImportRows {
        edges: Vec::new(),
        names: vec![Name {
            source: path("src/a.x"),
            written: WrittenPath(written.into()),
            binding: NameBinding::Binds(ModuleName("thing".into())),
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
    let scope = |start, end, kind| UseScope {
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
            binding: NameBinding::Binds(ModuleName("contract".into())),
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
        binding: NameBinding::Unbound,
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
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
