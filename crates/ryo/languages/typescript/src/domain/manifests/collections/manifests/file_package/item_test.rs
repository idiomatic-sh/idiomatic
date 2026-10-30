//! Nearest package ownership includes manifests and nonmember packages.

use super::item::FilePackageItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{WORKSPACE, read};
use crate::fixtures::workspaces::{UNNAMED_NESTED, UNNAMED_ROOT};

type Files = &'static [(&'static str, &'static str)];
type Input = (Files, &'static [(u32, u32)]);
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_file_uses_its_nearest_package"),
        word: Word::Succeeds,
        input: || {
            (
                WORKSPACE,
                &[
                    (0, 1),
                    (1, 1),
                    (2, 3),
                    (3, 3),
                    (4, 3),
                    (5, 5),
                    (6, 6),
                    (7, 6),
                ],
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unnamed_root_owns_its_files"),
        word: Word::Succeeds,
        input: || (UNNAMED_ROOT, &[(0, 1), (1, 1), (2, 3), (3, 3)]),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unnamed_nested_package_owns_its_directory"),
        word: Word::Succeeds,
        input: || (UNNAMED_NESTED, &[(0, 1), (1, 1), (2, 2), (3, 3)]),
        expect: Expect::Value(true),
    },
];
fn subject((files, expected): Input) -> Value {
    let (written, _) = read::<FilePackageItem>(files);
    written.partial == 0
        && written
            .rows
            .iter()
            .map(|row| (row.path.0, row.package.0))
            .collect::<Vec<_>>()
            == expected
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
