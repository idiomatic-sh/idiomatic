//! The `project_namespace_membership` item's cases, every row in full: each dotted prefix of an absolute import, and whether the
//! project holds it as a module, a package or a namespace.

use super::item::ProjectNamespaceMembershipItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{Reading, Show, Shown, WORKSPACE, reading};
use ryo::languages;

/// The manifests and files of a repository: each file with its path and its text.
type Input = &'static [(&'static str, &'static str)];
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("files_that_import_nothing_have_no_membership"),
        word: Word::Succeeds,
        input: || WORKSPACE,
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

impl Show for languages::ProjectNamespaceMembershipRow {
    /// The importing file, then the root, the prefix, its membership and its kind.
    fn show(&self, shown: &Shown<'_>) -> String {
        format!(
            "{} {} {} {} {}",
            shown.file(self.file),
            shown.name(self.root),
            shown.name(self.path),
            shown.name(self.membership),
            shown.name(self.kind)
        )
    }
}

fn subject(files: Input) -> Value {
    reading::<ProjectNamespaceMembershipItem>(files)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
