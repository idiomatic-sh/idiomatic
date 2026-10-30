//! The Poetry tables' cases: each table of dependencies gives its kind, the interpreter is no dependency, and a table form gives its
//! optionality, its marker and its directory.

use super::super::adapter::ManifestReadingPoetryAdapter;
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
        name: CaseName("each_table_of_dependencies_gives_its_kind_and_the_interpreter_is_none"),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[tool.poetry]\nname = \"P\"\n[tool.poetry.dependencies]\npython = \"^3.9\"\nrequests = \"^2\"\n[tool.poetry.group.test.dependencies]\npytest = \"*\"\n[tool.poetry.group.main.dependencies]\nrich = \"*\"\n[tool.poetry.dev-dependencies]\nblack = \"*\"\n",
            )]
        },
        expect: Expect::Value(
            "P |  |  |  | requests Normal Required <^2>, rich Normal Required <*>, pytest Dev Required <*>, black Dev Required <*>",
        ),
    },
    Case {
        name: CaseName("a_table_form_gives_its_optionality_its_marker_and_its_directory"),
        word: Word::Succeeds,
        input: || {
            &[(
                "apps/pyproject.toml",
                "[tool.poetry.dependencies]\nlib = { path = \"../libs/lib\", develop = true }\nextra = { version = \"1\", optional = true, extras = [\"x\"] }\nwin = { version = \"1\", markers = \"sys_platform == 'win32'\" }\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | extra Normal Optional <1>!, lib Normal Required @libs/lib Path, win ; sys_platform == 'win32' Normal Required <1>!",
        ),
    },
    Case {
        name: CaseName(
            "a_platform_is_the_sys_platform_marker_and_holds_with_the_markers_beside_it",
        ),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[tool.poetry.dependencies]\nlin = { version = \"1\", platform = \"linux\" }\nboth = { version = \"1\", platform = \"darwin\", markers = \"python_version >= '3.10'\" }\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | both ; (python_version >= '3.10') and sys_platform == 'darwin' Normal Required <1>!, lin ; sys_platform == 'linux' Normal Required <1>!",
        ),
    },
    Case {
        name: CaseName(
            "a_list_of_tables_is_one_requirement_each_with_the_versions_of_python_it_is_for",
        ),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[tool.poetry.dependencies]\nnumpy = [{ version = \"1\", python = \"<3.10\" }, { version = \"2\", python = \">=3.10\", markers = \"os_name == 'posix'\" }]\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | numpy Normal Required <1>! python=<3.10, numpy ; os_name == 'posix' Normal Required <2>! python=>=3.10",
        ),
    },
    Case {
        name: CaseName(
            "a_git_dependency_keeps_the_branch_the_tag_or_the_revision_it_names_by_its_key",
        ),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[tool.poetry.dependencies]\nb = { git = \"https://example.com/b.git\", branch = \"main\" }\nr = { git = \"https://example.com/r.git\", rev = \"0123456\" }\nt = { git = \"https://example.com/t.git\", tag = \"v1.0.0\" }\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | b Normal Required Git branch=main, r Normal Required Git rev=0123456, t Normal Required Git tag=v1.0.0",
        ),
    },
    Case {
        name: CaseName("a_path_outside_the_repository_is_unresolvable"),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[tool.poetry.dependencies]\nlib = { path = \"../lib\" }\n",
            )]
        },
        expect: Expect::Value(" |  |  |  | lib Normal Required @? Path"),
    },
    Case {
        name: CaseName("the_pep_tables_declare_nothing"),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[project]\nname = \"p\"\ndependencies = [\"x\"]\n[dependency-groups]\ndev = [\"y\"]\n",
            )]
        },
        expect: Expect::Value(" |  |  |  | "),
    },
    Case {
        name: CaseName("a_dependency_says_its_registry_its_path_its_git_repository_or_its_url"),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[tool.poetry.dependencies]\na = \"^1\"\nb = { path = \"libs/b\" }\nc = { git = \"https://example.com/c.git\" }\nd = { url = \"https://example.com/d.whl\" }\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | a Normal Required <^1>, b Normal Required @libs/b Path, c Normal Required Git, d Normal Required Url",
        ),
    },
];

fn subject(files: Input) -> Result<Value, Failure> {
    declared_by(|_| ManifestReadingPoetryAdapter, files)
        .map(|line| &*Box::leak(line.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
