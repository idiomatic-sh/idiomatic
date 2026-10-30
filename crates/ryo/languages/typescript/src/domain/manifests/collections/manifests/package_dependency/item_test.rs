//! Every dependency column, npm dependency kind, alias and project-reference edge.

use super::item::PackageDependencyItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{WORKSPACE, read, shown};
use crate::fixtures::workspaces::{PEERS, UNNAMED_NESTED, UNNAMED_REFERENCE, UNNAMED_ROOT};
use ryo::languages::{DependencyKind, DependencyOrigin, Optionality};

type Files = &'static [(&'static str, &'static str)];
/// The source and target files (`NO_FILE`'s number for a package outside the repository), the binding, the kind, the optionality and
/// the origin of a row.
type Edge = (
    u32,
    u32,
    &'static str,
    DependencyKind,
    Optionality,
    DependencyOrigin,
);
type Input = (Files, &'static [Edge]);
/// Whether the rows are the edges expected, and how many causes the relation is read in part for.
type Value = (bool, usize);
type Failure = ();
const WORKSPACE_EDGES: &[Edge] = &[
    (
        1,
        3,
        "alias",
        DependencyKind::Normal,
        Optionality::Required,
        DependencyOrigin::Registry,
    ),
    (
        1,
        3,
        "core",
        DependencyKind::Normal,
        Optionality::Required,
        DependencyOrigin::Workspace,
    ),
    (
        1,
        u32::MAX,
        "outside",
        DependencyKind::Normal,
        Optionality::Required,
        DependencyOrigin::Registry,
    ),
    (
        1,
        3,
        "core",
        DependencyKind::Dev,
        Optionality::Required,
        DependencyOrigin::Registry,
    ),
    (
        1,
        3,
        "core",
        DependencyKind::Peer,
        Optionality::Required,
        DependencyOrigin::Registry,
    ),
    (
        1,
        6,
        "util",
        DependencyKind::Normal,
        Optionality::Optional,
        DependencyOrigin::Registry,
    ),
    (
        6,
        3,
        "core",
        DependencyKind::Normal,
        Optionality::Required,
        DependencyOrigin::Path,
    ),
];
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "dependencies_keep_kinds_aliases_references_and_origins_and_one_outside_has_no_target",
        ),
        word: Word::Succeeds,
        input: || (WORKSPACE, WORKSPACE_EDGES),
        expect: Expect::Value((true, 0)),
    },
    Case {
        name: CaseName("unnamed_root_emits_its_dependencies"),
        word: Word::Succeeds,
        input: || {
            (
                UNNAMED_ROOT,
                &[(
                    3,
                    1,
                    "core",
                    DependencyKind::Normal,
                    Optionality::Required,
                    DependencyOrigin::Registry,
                )],
            )
        },
        expect: Expect::Value((true, 0)),
    },
    Case {
        name: CaseName("unnamed_nested_package_emits_its_dependencies"),
        word: Word::Succeeds,
        input: || {
            (
                UNNAMED_NESTED,
                &[(
                    1,
                    2,
                    "core",
                    DependencyKind::Normal,
                    Optionality::Required,
                    DependencyOrigin::Registry,
                )],
            )
        },
        expect: Expect::Value((true, 0)),
    },
    Case {
        name: CaseName(
            "a_project_reference_into_an_unnamed_package_invents_no_name_and_is_read_in_part",
        ),
        word: Word::Succeeds,
        input: || (UNNAMED_REFERENCE, &[]),
        expect: Expect::Value((true, 1)),
    },
    Case {
        name: CaseName("peer_optionality_follows_peer_dependencies_meta"),
        word: Word::Succeeds,
        input: || {
            (
                PEERS,
                &[
                    (
                        2,
                        0,
                        "bare",
                        DependencyKind::Peer,
                        Optionality::Required,
                        DependencyOrigin::Registry,
                    ),
                    (
                        2,
                        1,
                        "core",
                        DependencyKind::Peer,
                        Optionality::Optional,
                        DependencyOrigin::Registry,
                    ),
                    (
                        2,
                        3,
                        "req",
                        DependencyKind::Peer,
                        Optionality::Required,
                        DependencyOrigin::Registry,
                    ),
                ],
            )
        },
        expect: Expect::Value((true, 0)),
    },
];

fn subject((files, expected): Input) -> Value {
    let (written, symbols) = read::<PackageDependencyItem>(files);
    let rows = written
        .rows
        .iter()
        .map(|row| {
            (
                row.source.0,
                row.target.0,
                shown(&symbols, row.binding),
                shown(&symbols, row.package),
                row.kind,
                row.optional,
                row.origin,
                shown(&symbols, row.condition),
            )
        })
        .collect::<Vec<_>>();
    let expected = expected
        .iter()
        .map(|&(source, target, name, kind, optional, origin)| {
            (
                source,
                target,
                name.to_owned(),
                if name == "alias" { "core" } else { name }.to_owned(),
                kind,
                optional,
                origin,
                String::new(),
            )
        })
        .collect::<Vec<_>>();
    (rows == expected, written.partial)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
