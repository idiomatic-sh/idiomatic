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

#[test]
fn invalid_author_retains_valid_dependency_projection() {
    use super::super::schemas::read;
    use crate::domain::manifest_validation::types::{MetadataDemand, Phase};
    let text = "[project]\nname = 'example'\nversion = '1.0'\nauthors = [{name = 'author', email = 1}]\ndependencies = ['requests>=2; python_version > \"3.8\"']\n";
    let result = read(text, MetadataDemand::Both);
    let declaration = result
        .declaration
        .expect("projection requested")
        .expect("unrelated metadata must not erase dependencies");
    assert_eq!(declaration.requirements.len(), 1);
    assert_eq!(
        declaration.requirements[0].written.0.as_ref(),
        "requests>=2; python_version > \"3.8\""
    );
    let validation = result.validation.expect("requested metadata validation");
    assert_eq!(validation.diagnostics.len(), 1);
    assert_eq!(validation.diagnostics[0].phase, Phase::Validation);
    assert!(validation.incomplete.is_none());
}

#[test]
fn invalid_dependency_shape_never_publishes_a_fabricated_requirement() {
    use super::super::schemas::read;
    use crate::domain::manifest_validation::types::MetadataDemand;
    let result = read(
        "[project]\nname = 'example'\nversion = '1.0'\ndependencies = [1]\n",
        MetadataDemand::Both,
    );
    assert!(result.declaration.expect("projection requested").is_err());
    assert!(
        !result
            .validation
            .expect("requested validation")
            .diagnostics
            .is_empty()
    );
}

#[test]
fn cloned_document_preserves_unicode_error_span_without_reparsing() {
    use serde::Deserialize;
    let text = "[project]\nname = 'example'\nversion = '1.0'\nauthors = [{name = 'é\\\\author', email = 1}]\n";
    let document = toml::de::DeTable::parse(text).expect("valid TOML");
    let first =
        pyproject_toml::PyProjectToml::deserialize(toml::de::Deserializer::from(document.clone()))
            .expect_err("invalid email");
    let second = pyproject_toml::PyProjectToml::deserialize(toml::de::Deserializer::from(document))
        .expect_err("invalid email");
    assert_eq!(first.span(), second.span());
    assert_eq!(first.message(), second.message());
    let span = first
        .span()
        .expect("schema error retains a real source range");
    let expected_start = text
        .find("[{")
        .expect("the invalid author belongs to the authors array");
    let expected_end = text.rfind(']').expect("authors array closes") + 1;
    assert_eq!(span, expected_start..expected_end);
    assert_eq!(text.get(span), text.get(expected_start..expected_end));
}

#[test]
fn invalid_requirement_text_is_not_a_projected_dependency() {
    use super::super::schemas::read;
    use crate::domain::manifest_validation::types::MetadataDemand;
    let result = read(
        "[project]\nname = 'example'\nversion = '1.0'\ndependencies = ['...']\n",
        MetadataDemand::Both,
    );
    assert!(result.declaration.expect("projection requested").is_err());
    assert_eq!(
        result
            .validation
            .expect("metadata requested")
            .diagnostics
            .len(),
        1
    );
}

#[test]
fn metadata_only_does_not_project_rejected_dependencies() {
    use super::super::schemas::read;
    use crate::domain::manifest_validation::types::MetadataDemand;
    let result = read(
        "[project]\nname = 'example'\nversion = '1.0'\ndependencies = [1]\n",
        MetadataDemand::Diagnostics,
    );
    assert!(result.declaration.is_none());
    assert_eq!(
        result
            .validation
            .expect("requested metadata read")
            .diagnostics
            .len(),
        1
    );
}
