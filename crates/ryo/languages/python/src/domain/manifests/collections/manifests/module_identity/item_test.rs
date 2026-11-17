//! The `module_identity` item's cases, every row in full: the module path, the module, its package and its package root of each
//! source file the project context places, and never two rows of one file, so `file` is the relation's key.

use super::item::ModuleIdentityItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{IMPORTS, MONOREPO, Reading, Show, Shown, WORKSPACE, reading};
use ryo::languages;

/// The manifests and files of a repository: each file with its path and its text.
type Input = &'static [(&'static str, &'static str)];
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
/// A file the item wrote two rows of: its path.
#[derive(Debug, PartialEq, Eq)]
struct RepeatedFile(String);
type Failure = RepeatedFile;

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
            causes: &[],
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
            causes: &[],
        }),
    },
    Case {
        name: CaseName("each_file_of_a_workspace_has_one_identity_at_most"),
        word: Word::Succeeds,
        input: || WORKSPACE,
        expect: Expect::Value(Reading {
            rows: &[
                "packages/lib_one/src/lib_one/__init__.py \"lib_one.__init__\" \"__init__\" \"lib_one\" \"lib_one\"",
            ],
            causes: &[],
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

/// What the item writes for `files`, refused when it writes two rows of one file.
fn subject(files: Input) -> Result<Value, Failure> {
    let read = reading::<ModuleIdentityItem>(files);
    let mut seen = std::collections::BTreeSet::new();
    for row in read.rows {
        let file = row.split(' ').next().unwrap_or(row);
        if !seen.insert(file) {
            return Err(RepeatedFile(file.to_owned()));
        }
    }
    Ok(read)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
