//! The `uv` tables' cases: a workspace gives its members and its exclusions, a source its directory.

use super::declared;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
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
        name: CaseName("a_uv_workspace_gives_its_members_and_its_exclusions"),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[tool.uv.workspace]\nmembers = [\"packages/*\"]\nexclude = [\"packages/old\"]\n",
            )]
        },
        expect: Expect::Value(" | packages/* | packages/old |  | "),
    },
    Case {
        name: CaseName("a_path_source_sends_the_package_to_its_directory_from_the_manifest"),
        word: Word::Succeeds,
        input: || {
            &[(
                "apps/web/pyproject.toml",
                "[tool.uv.sources]\nlib = { path = \"../../libs/lib\" }\nother = [{ path = \"./vendor/other\", marker = \"sys_platform == 'win32'\" }]\n",
            )]
        },
        expect: Expect::Value(
            " |  |  | lib=libs/lib,other=apps/web/vendor/other ; sys_platform == 'win32' | ",
        ),
    },
    Case {
        name: CaseName("a_workspace_source_names_no_directory"),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[tool.uv.sources]\nlib = { workspace = true }\nremote = { git = \"https://example.com/remote\" }\n",
            )]
        },
        expect: Expect::Value(" |  |  | lib=?:Workspace,remote=?:Git | "),
    },
    Case {
        name: CaseName(
            "a_path_source_outside_the_repository_sends_the_package_away_from_the_repository",
        ),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[tool.uv.sources]\nlib = { path = \"../elsewhere\" }\n",
            )]
        },
        expect: Expect::Value(" |  |  | lib=? | "),
    },
    Case {
        name: CaseName("the_pep_tables_declare_nothing"),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[project]\nname = \"p\"\ndependencies = [\"x\"]\n",
            )]
        },
        expect: Expect::Value(" |  |  |  | "),
    },
    Case {
        name: CaseName(
            "a_source_says_its_registry_its_path_its_git_repository_at_its_revision_its_url_or_the_workspace",
        ),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[tool.uv.sources]\na = { index = \"internal\" }\nb = { path = \"libs/b\" }\nc = { git = \"https://example.com/c.git\", tag = \"v1.0.0\" }\nd = { workspace = true }\ne = { url = \"https://example.com/e.whl\" }\n",
            )]
        },
        expect: Expect::Value(
            " |  |  | a=?:Registry,b=libs/b,c=?:Git tag=v1.0.0,d=?:Workspace,e=?:Url | ",
        ),
    },
    Case {
        name: CaseName("the_override_dependencies_are_kept_as_written"),
        word: Word::Succeeds,
        input: || {
            &[(
                "pyproject.toml",
                "[tool.uv]\noverride-dependencies = [\"werkzeug==2.3.0\", \"numpy<2 ; python_version < '3.10'\"]\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  |  | override werkzeug==2.3.0 | override numpy<2 ; python_version < '3.10'",
        ),
    },
    Case {
        name: CaseName("a_tool_that_is_not_a_table_is_a_malformed_manifest_the_reader_refuses"),
        word: Word::Refuses,
        input: || &[("pyproject.toml", "tool = 1\n")],
        expect: Expect::Refusal(names_the_tool_shape),
    },
    Case {
        name: CaseName("a_uv_entry_that_is_not_a_table_is_a_malformed_manifest_the_reader_refuses"),
        word: Word::Refuses,
        input: || &[("pyproject.toml", "[tool]\nuv = 1\n")],
        expect: Expect::Refusal(names_the_uv_shape),
    },
];

/// Whether the refusal is the typed `Unreadable` that names the shape of `[tool]` the manifest broke.
fn names_the_tool_shape(failure: &Failure) -> Selection {
    match failure {
        ManifestReadingError::Unreadable { reason } if reason.contains("ToolSchema") => {
            Selection::Matches
        }
        ManifestReadingError::Unreadable { .. } => Selection::Differs,
    }
}

/// Whether the refusal is the typed `Unreadable` that names the shape of `[tool.uv]` the manifest broke.
fn names_the_uv_shape(failure: &Failure) -> Selection {
    match failure {
        ManifestReadingError::Unreadable { reason } if reason.contains("UvSchema") => {
            Selection::Matches
        }
        ManifestReadingError::Unreadable { .. } => Selection::Differs,
    }
}

fn subject(files: Input) -> Result<Value, Failure> {
    declared_with(|path, text, _| declared(path, text), files)
        .map(|line| &*Box::leak(line.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
