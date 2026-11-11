//! The `project_namespace_membership` item's cases, every row in full: each dotted prefix of an absolute import, and whether the
//! project holds it as a module, a package or a namespace.

use super::item::ProjectNamespaceMembershipItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{IMPORTS, Reading, Show, Shown, WORKSPACE, reading};
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
    Case {
        name: CaseName("with_no_search_scope_each_prefix_of_an_absolute_import_is_unknown_and_the_relation_partial"),
        word: Word::Succeeds,
        input: || IMPORTS,
        expect: Expect::Value(Reading {
            rows: &[
                "app.py \"lib_one\" \"lib_one\" \"unknown\" \"unknown\"",
                "app.py \"lib_one\" \"lib_one.absent\" \"unknown\" \"unknown\"",
                "app.py \"lib_one\" \"lib_one.run\" \"unknown\" \"unknown\"",
                "app.py \"lib_one\" \"lib_one.workers\" \"unknown\" \"unknown\"",
                "app.py \"ns\" \"ns\" \"unknown\" \"unknown\"",
                "app.py \"ns\" \"ns.part\" \"unknown\" \"unknown\"",
                "app.py \"ns\" \"ns.part.thing\" \"unknown\" \"unknown\"",
                "app.py \"os\" \"os\" \"unknown\" \"unknown\"",
                "app.py \"tools\" \"tools\" \"unknown\" \"unknown\"",
                "app.py \"tools\" \"tools.cli\" \"unknown\" \"unknown\"",
                "tools/cli.py \"app\" \"app\" \"unknown\" \"unknown\"",
            ],
            causes: &[
                "project namespace membership has an unobserved search entry",
            ],
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
