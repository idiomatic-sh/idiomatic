//! The `Pipfile`'s cases: each table gives its kind, a table form its marker and its directory.

use super::declared;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use crate::outbound::manifest_reading::fixtures::shown::{Files, declared_with};

/// The manifests of a case: the first is read.
type Input = Files;
/// What it declares, as one line.
type Value = &'static str;
type Failure = ManifestReadingError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("packages_are_normal_and_dev_packages_are_dev"),
        word: Word::Succeeds,
        input: || {
            &[(
                "Pipfile",
                "[[source]]\nurl = \"https://pypi.org/simple\"\n[packages]\nrequests = \"*\"\nflask = {version = \">=2\", extras = [\"async\"]}\n[dev-packages]\npytest = \"*\"\n[requires]\npython_version = \"3.12\"\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | flask Normal Required <>=2>, requests Normal Required <*>, pytest Dev Required <*>",
        ),
    },
    Case {
        name: CaseName("a_table_form_gives_its_marker_and_its_directory"),
        word: Word::Succeeds,
        input: || {
            &[(
                "apps/Pipfile",
                "[packages]\nlib = {path = \"../libs/lib\", editable = true}\nwin = {version = \"*\", markers = \"sys_platform == 'win32'\"}\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | lib Normal Required @libs/lib Path, win ; sys_platform == 'win32' Normal Required <*>",
        ),
    },
    Case {
        name: CaseName("a_pipfile_with_no_package_tables_declares_nothing"),
        word: Word::Succeeds,
        input: || &[("Pipfile", "[requires]\npython_version = \"3.12\"\n")],
        expect: Expect::Value(" |  |  |  | "),
    },
    Case {
        name: CaseName(
            "a_package_says_its_registry_its_path_its_git_repository_at_its_ref_or_its_url",
        ),
        word: Word::Succeeds,
        input: || {
            &[(
                "Pipfile",
                "[packages]\na = \"*\"\nb = { path = \"libs/b\" }\nc = { git = \"https://example.com/c.git\", ref = \"v3\" }\nd = { file = \"https://example.com/d.whl\" }\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | a Normal Required <*>, b Normal Required @libs/b Path, c Normal Required Git rev=v3, d Normal Required Url",
        ),
    },
];

fn subject(files: Input) -> Result<Value, Failure> {
    declared_with(|path, text, _| declared(path, text), files)
        .map(|line| &*Box::leak(line.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
