//! The `module_identity` item's cases, every row in full: the module path, the module, its package and its package root of each
//! source file the project context places.

use super::item::ModuleIdentityItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{IMPORTS, MONOREPO, Reading, Show, Shown, reading};
use ryo::languages;

/// The manifests and files of a repository: each file with its path and its text.
type Input = &'static [(&'static str, &'static str)];
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_package_module_has_its_identity_and_a_root_script_none"),
        word: Word::Succeeds,
        input: || MONOREPO,
        expect: Expect::Value(Reading {
            rows: &[
                "packages/lib_one/src/lib_one/__init__.py \"lib_one.__init__\" \"__init__\" \"lib_one\" \"lib_one\"",
            ],
            causes: &[
            ],
        }),
    },
    Case {
        name: CaseName("each_module_of_a_regular_package_has_its_identity"),
        word: Word::Succeeds,
        input: || IMPORTS,
        expect: Expect::Value(Reading {
            rows: &[
                "packages/lib_one/src/lib_one/__init__.py \"lib_one.__init__\" \"__init__\" \"lib_one\" \"lib_one\"",
                "packages/lib_one/src/lib_one/workers.py \"lib_one.workers\" \"workers\" \"lib_one\" \"lib_one\"",
                "tools/__init__.py \"tools.__init__\" \"__init__\" \"tools\" \"tools\"",
                "tools/cli.py \"tools.cli\" \"cli\" \"tools\" \"tools\"",
            ],
            causes: &[
            ],
        }),
    },
];

impl Show for languages::ModuleIdentityRow {
    /// The file, then its module path, module, package and package root.
    fn show(&self, shown: &Shown<'_>) -> String {
        format!(
            "{} {} {} {} {}",
            shown.file(self.file),
            shown.name(self.path),
            shown.name(self.module_name),
            shown.name(self.package_name),
            shown.name(self.package_root_name)
        )
    }
}

fn subject(files: Input) -> Value {
    reading::<ModuleIdentityItem>(files)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
