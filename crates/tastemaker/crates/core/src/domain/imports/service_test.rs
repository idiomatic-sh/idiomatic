//! What the `imports` service reads of the import relations and what an entry's `imports` refuses of them.

use std::collections::{BTreeMap, BTreeSet};

use super::service::{by_source, refused};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::comments::types::ByteSpan;
use crate::domain::entries::types::{EntryIdentity, ImportDefault};
use crate::domain::findings::types::ByteOffset;
use crate::domain::import_rows::service::imports;
use crate::domain::imports::types::{
    AllowItem, EntryImports, Import, ImportPlace, ImportRows, Member, Name, Owner, Package,
    PatternMatch, ScopeImports, Target, WrittenPath,
};
use crate::domain::module_paths::service::prefixes;
use crate::domain::paths::types::RepoPath;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The imports `src/a.txt` makes: a `use` of a sibling module, one of a package of the repository, one of a module
    /// outside it, and a path written inline that names a sibling.
    Read,
    /// What a `default = "deny"` entry allowing `types` and `std::*` refuses of those imports.
    Refused,
}

type Input = Question;
type Value = String;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_import_is_read_with_what_it_reaches"),
        word: Word::Succeeds,
        input: || Question::Read,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_deny_entry_refuses_what_its_allow_does_not_list"),
        word: Word::Succeeds,
        input: || Question::Refused,
        expect: Expect::Value(String::new()),
    },
];

fn path(text: &str) -> RepoPath {
    RepoPath(text.into())
}

fn span(start: usize, end: usize) -> ImportPlace {
    ImportPlace::At(ByteSpan {
        start: ByteOffset(start),
        end: ByteOffset(end),
    })
}

/// `src/a.txt`: `use crate::types::T;` (4..19), `use other::X;` (24..32), `use std::fmt;` (37..45), and `crate::b::f()`
/// written inline (60..72).
fn rows() -> ImportRows {
    ImportRows {
        edges: vec![
            Import {
                source: path("src/a.txt"),
                span: span(4, 19),
                written: "crate::types::T".into(),
                target: Target::File(path("src/types.txt")),
            },
            Import {
                source: path("src/a.txt"),
                span: span(60, 72),
                written: "crate::b::f".into(),
                target: Target::File(path("src/b.txt")),
            },
        ],
        names: vec![
            Name {
                source: path("src/a.txt"),
                written: "crate::types::T".into(),
                span: span(4, 19),
            },
            Name {
                source: path("src/a.txt"),
                written: "other::X".into(),
                span: span(24, 32),
            },
            Name {
                source: path("src/a.txt"),
                written: "std::fmt".into(),
                span: span(37, 45),
            },
        ],
        packages: vec![Package {
            source: path("Cargo.toml"),
            target: path("other/Cargo.toml"),
            binding: "other".into(),
        }],
        owners: vec![Owner {
            path: path("src/a.txt"),
            package: path("Cargo.toml"),
        }],
    }
}

fn read() -> Vec<Import> {
    imports(&rows())
}

fn subject(question: Input) -> Result<Value, Failure> {
    let shown = |found: &[Import]| {
        found
            .iter()
            .map(|import| match &import.target {
                Target::File(file) => format!("{} -> {}", import.written, file.0),
                Target::Module => format!("{} -> module", import.written),
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    let (said, expected) = match question {
        Question::Read => (
            {
                let found = read();
                if found.is_empty() {
                    return Err("nothing was read".to_owned());
                }
                shown(&found)
            },
            "crate::types::T -> src/types.txt; other::X -> other/Cargo.toml; std::fmt -> module; crate::b::f -> src/b.txt"
                .to_owned(),
        ),
        Question::Refused => {
            let found = read();
            let made = by_source(&found, &BTreeSet::new());
            let scope = ScopeImports {
                names: BTreeSet::from(["types".into(), "module".into()]),
                entries_of: BTreeMap::from([
                    (path("src/types.txt"), vec!["types".into()]),
                    (path("src/a.txt"), vec!["module".into()]),
                ]),
                checks: vec![EntryImports {
                    identity: EntryIdentity("p/module".to_owned()),
                    default: ImportDefault::Deny,
                    allow: vec!["types".into(), "std::*".into()],
                    members: vec![Member {
                        instance: "src/a.txt".into(),
                        files: BTreeSet::from([path("src/a.txt")]),
                        extra: vec!["other".into()],
                    }],
                }],
            };
            let matches = |item: &AllowItem, written: &WrittenPath| {
                let hit = prefixes(written).iter().any(|prefix| {
                    let item = item.0.replace("::", "/");
                    item.strip_suffix("/*").map_or(item == *prefix.0, |head| {
                        prefix.0.starts_with(&format!("{head}/"))
                    })
                });
                if hit {
                    PatternMatch::Matches
                } else {
                    PatternMatch::Misses
                }
            };
            let listed = refused(&scope, &made, &matches)
                .iter()
                .map(|each| each.import.written.0.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            (listed, "crate::b::f".to_owned())
        }
    };
    Ok(if said == expected {
        String::new()
    } else {
        said
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
