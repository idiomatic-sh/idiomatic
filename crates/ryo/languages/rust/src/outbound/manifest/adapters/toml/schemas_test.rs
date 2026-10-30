//! The accepted and the refused inputs of the `toml` adapter's schemas.

use super::{ConfigSchema, ManifestSchema, ManifestSchemaError};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::types::Section;
use crate::domain::targets::types::Config;
use crate::domain::targets::types::Manifest;
use crate::domain::targets::types::Replacement;

/// The shape a row converts.
#[derive(Clone, Copy)]
enum Scenario {
    /// A `Cargo.toml`.
    Manifest(&'static str),
    /// A `.cargo/config.toml`.
    Configuration(&'static str),
}

/// What a row converts: the text of a manifest, or of a configuration.
type Input = Scenario;
/// The dependencies the manifest declares, one each as `key kind version path source origin`, or the directory the configuration names.
type Value = &'static str;
type Failure = ManifestSchemaError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_package_with_two_tables_of_dependencies_gives_both"),
        word: Word::Succeeds,
        input: || {
            Scenario::Manifest(
                "[package]\nname = \"a\"\n[dependencies]\nx = \"1\"\n[dev-dependencies]\ny = { path = \"../y\" }\n",
            )
        },
        expect: Expect::Value("x Normal 1 - crates-io Registry; y Dev - ../y crates-io Path |"),
    },
    Case {
        name: CaseName("a_dependency_says_its_registry_its_path_or_its_git_repository"),
        word: Word::Succeeds,
        input: || {
            Scenario::Manifest(
                "[package]\nname = \"a\"\n[dependencies]\nserde_json = \"1\"\nlocal = { path = \"../local\" }\nremote = { git = \"https://example.com/remote\" }\nmirrored = { version = \"1\", registry = \"mirror\" }\n",
            )
        },
        expect: Expect::Value(
            "local Normal - ../local crates-io Path; mirrored Normal 1 - mirror Registry; remote Normal - - https://example.com/remote Git; serde_json Normal 1 - crates-io Registry |",
        ),
    },
    Case {
        name: CaseName(
            "a_git_dependency_keeps_the_branch_the_tag_or_the_revision_it_names_by_its_key",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::Manifest(
                "[package]\nname = \"a\"\n[dependencies]\nb = { git = \"https://example.com/b\", branch = \"main\" }\nr = { git = \"https://example.com/r\", rev = \"0123456\" }\nt = { git = \"https://example.com/t\", tag = \"v1.0.0\" }\n",
            )
        },
        expect: Expect::Value(
            "b Normal - - https://example.com/b Git branch=main; r Normal - - https://example.com/r Git rev=0123456; t Normal - - https://example.com/t Git tag=v1.0.0 |",
        ),
    },
    Case {
        name: CaseName(
            "a_root_patches_packages_into_directories_and_from_git_repositories_for_their_source",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::Manifest(
                "[workspace]\n[workspace.dependencies]\nz = { git = \"https://example.com/z\" }\n[patch.crates-io]\nx = { path = \"x\" }\nw = { git = \"https://example.com/w\" }\n[patch.'https://example.com/z']\nz = { path = \"vendor/z\", package = \"zed\" }\n[replace]\n\"r:1.0.0\" = { path = \"r\" }\n",
            )
        },
        expect: Expect::Value(
            " | crates-io w https://example.com/w; crates-io x x; https://example.com/z zed vendor/z; crates-io r r",
        ),
    },
    Case {
        name: CaseName("a_manifest_with_neither_a_package_nor_a_workspace_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Manifest("[dependencies]\nx = \"1\"\n"),
        expect: Expect::Error(ManifestSchemaError::Neither),
    },
    Case {
        name: CaseName("a_configuration_names_a_directory"),
        word: Word::Succeeds,
        input: || Scenario::Configuration("[build]\ntarget-dir = \"out\"\n"),
        expect: Expect::Value("out"),
    },
    Case {
        name: CaseName("a_configuration_with_an_empty_target_directory_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Configuration("[build]\ntarget-dir = \"\"\n"),
        expect: Expect::Error(ManifestSchemaError::EmptyTargetDir),
    },
    Case {
        name: CaseName("a_package_without_a_name_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Manifest("[package]\nversion = \"1\"\n"),
        expect: Expect::Error(ManifestSchemaError::Unnamed),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Manifest(text) => {
            let schema: ManifestSchema = toml::from_str(text).unwrap_or_default();
            Manifest::try_from(schema).map(|manifest| {
                let dependencies = manifest
                    .dependencies
                    .iter()
                    .map(|each| {
                        let version = match &each.version {
                            Section::Present(version) => version.0.to_string(),
                            Section::Absent => "-".to_owned(),
                        };
                        let path = match &each.path {
                            Section::Present(path) => path.0.to_string(),
                            Section::Absent => "-".to_owned(),
                        };
                        // A revision is shown only where the table writes one, by its key.
                        let checkout: String = [
                            ("branch", &each.checkout.branch),
                            ("tag", &each.checkout.tag),
                            ("rev", &each.checkout.rev),
                        ]
                        .into_iter()
                        .filter_map(|(key, revision)| match revision {
                            Section::Present(revision) => Some(format!(" {key}={}", revision.0)),
                            Section::Absent => None,
                        })
                        .collect();
                        format!(
                            "{} {:?} {version} {path} {} {:?}{checkout}",
                            each.key, each.kind, each.source.0, each.origin
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                let patches = manifest
                    .patches
                    .iter()
                    .map(|each| {
                        let replacement = match &each.replacement {
                            Replacement::Directory(path) => &path.0,
                            Replacement::Repository(url) => &url.0,
                        };
                        format!(" {} {} {replacement}", each.source.0, each.package)
                    })
                    .collect::<Vec<_>>()
                    .join(";");
                let shown = format!("{dependencies} |{patches}");
                &*Box::leak(shown.into_boxed_str())
            })
        }
        Scenario::Configuration(text) => {
            let schema: ConfigSchema = toml::from_str(text).unwrap_or_default();
            Config::try_from(schema).map(|config| match config.target_dir {
                Section::Present(directory) => &*Box::leak(directory.0),
                Section::Absent => "",
            })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
