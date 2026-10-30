//! The PEP tables' cases: each table of requirements gives its kind, and a table of another package manager gives nothing.

use super::super::adapter::ManifestReadingPep621Adapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use crate::outbound::manifest_reading::fixtures::shown::{Files, declared_by};

/// The manifests of a case: the first is read.
type Input = Files;
/// What it declares, as one line.
type Value = &'static str;
type Failure = ManifestReadingError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_table_of_requirements_gives_its_kind"),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[project]\nname = \"A\"\ndependencies = [\"x>=1\"]\n[project.optional-dependencies]\ndocs = [\"y\"]\n[dependency-groups]\ndev = [\"z\", {include-group = \"docs\"}]\n[build-system]\nrequires = [\"w\"]\n",
            )]
        },
        expect: Expect::Value(
            "A |  |  |  | x>=1 Normal Required <>=1>, y Normal Optional, z Dev Required, w Build Required",
        ),
    },
    Case {
        name: CaseName("a_requirement_keeps_its_extras_and_its_marker_as_written"),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[project]\ndependencies = [\"requests[security]>=2 ; python_version >= '3.9'\"]\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | requests[security]>=2 ; python_version >= '3.9' Normal Required <>=2>",
        ),
    },
    Case {
        name: CaseName("an_included_group_is_counted_once_as_its_own"),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[dependency-groups]\ntest = [\"pytest\"]\nall = [{include-group = \"test\"}, \"coverage\"]\n",
            )]
        },
        expect: Expect::Value(" |  |  |  | coverage Dev Required, pytest Dev Required"),
    },
    Case {
        name: CaseName("the_tables_of_other_package_managers_declare_nothing"),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[tool.poetry]\nname = \"p\"\n[tool.poetry.dependencies]\npython = \"^3.9\"\nrequests = \"*\"\n[tool.uv.workspace]\nmembers = [\"a\"]\n",
            )]
        },
        expect: Expect::Value(" |  |  |  | "),
    },
    Case {
        name: CaseName(
            "a_direct_reference_says_its_path_its_git_repository_with_its_revision_or_its_url",
        ),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[project]\ndependencies = [\"a>=1\", \"b @ file:///abs/b\", \"c @ git+https://example.com/c.git@v2.0 ; os_name == 'posix'\", \"d @ https://example.com/d.whl\"]\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | a>=1 Normal Required <>=1>, b @ file:///abs/b Normal Required Path, c @ git+https://example.com/c.git@v2.0 ; os_name == 'posix' Normal Required Git rev=v2.0, d @ https://example.com/d.whl Normal Required Url",
        ),
    },
];

fn subject(files: Input) -> Result<Value, Failure> {
    declared_by(|_| ManifestReadingPep621Adapter, files)
        .map(|line| &*Box::leak(line.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
