//! The project configuration's shape (JSONC) and its conversion to references between packages, the configurations it extends, and
//! the paths its imports are found under.

use std::collections::BTreeMap;

use super::constants::{CONFIGURATION_EXTENSION, CURRENT};
use crate::domain::packages::service::{directory, relative};
use crate::domain::packages::types::{
    AliasPattern, Declared, Directory, PathAlias, RepositoryPath,
};
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use ryo::languages::ManifestFile;
use serde::Deserialize;

/// One entry of `references`: a project the configuration builds on.
#[derive(Deserialize)]
struct ReferenceSchema {
    /// The referenced project's directory or configuration file.
    path: String,
}

/// `extends`: one configuration, or several, applied in order.
#[derive(Deserialize)]
#[serde(untagged)]
enum ExtendsSchema {
    /// One configuration.
    One(String),
    /// Several configurations, applied in order.
    Many(Vec<String>),
}

/// The parts of `compilerOptions` that say where imports are found.
#[derive(Default, Deserialize)]
struct CompilerOptionsSchema {
    /// The directory non-relative imports are resolved from.
    #[serde(default, rename = "baseUrl")]
    base_url: Option<String>,
    /// The patterns imports are mapped through, each to the paths it may resolve to.
    #[serde(default)]
    paths: BTreeMap<String, Vec<String>>,
}

/// The parts of a project configuration the adapter reads.
#[derive(Default, Deserialize)]
struct ConfigSchema {
    /// The projects it references.
    #[serde(default)]
    references: Vec<ReferenceSchema>,
    /// The configurations it extends.
    #[serde(default)]
    extends: Option<ExtendsSchema>,
    /// Its compiler options.
    #[serde(default, rename = "compilerOptions")]
    compiler_options: CompilerOptionsSchema,
}

/// The configuration to read, as the adapter hands it over.
pub(super) struct ManifestSchema<'a> {
    /// The configuration's path and text.
    pub manifest: &'a ManifestFile,
}

impl TryFrom<ManifestSchema<'_>> for Declared {
    type Error = ManifestReadingError;

    #[expect(
        clippy::case_sensitive_file_extension_comparisons,
        reason = "The configuration filename is matched case sensitively, as TypeScript matches it"
    )]
    fn try_from(input: ManifestSchema<'_>) -> Result<Self, Self::Error> {
        let manifest = input.manifest;
        let base = directory(RepositoryPath(&manifest.path.0));
        let options = jsonc_parser::ParseOptions {
            allow_comments: true,
            allow_trailing_commas: true,
            allow_loose_object_property_names: false,
            allow_missing_commas: false,
            allow_single_quoted_strings: false,
            allow_hexadecimal_numbers: false,
            allow_unary_plus_numbers: false,
            allow_bare_decimal_point_numbers: false,
            allow_extended_string_escapes: false,
            allow_non_finite_numbers: false,
        };
        let config: ConfigSchema =
            jsonc_parser::parse_to_serde_value(&manifest.text.0, &options)
                .map_err(|error| ManifestReadingError::Unreadable(error.to_string().into()))?;
        let base_url = config
            .compiler_options
            .base_url
            .map(|url| relative(base, RepositoryPath(&url)));
        let from = base_url.as_ref().map_or(base, |url| Directory(&url.0));
        let extended = match config.extends {
            Some(ExtendsSchema::One(one)) => vec![one],
            Some(ExtendsSchema::Many(many)) => many,
            None => Vec::new(),
        };
        Ok(Declared {
            references: config
                .references
                .into_iter()
                .map(|reference| {
                    let path = if reference.path.ends_with(".json") {
                        reference.path
                    } else {
                        format!("{}/tsconfig.json", reference.path)
                    };
                    relative(base, RepositoryPath(&path))
                })
                .collect(),
            aliases: config
                .compiler_options
                .paths
                .into_iter()
                .map(|(pattern, targets)| PathAlias {
                    pattern: AliasPattern(pattern.into()),
                    targets: targets
                        .iter()
                        .map(|target| relative(from, RepositoryPath(target)))
                        .collect(),
                })
                .collect(),
            // A configuration a package publishes (`@tsconfig/node20/tsconfig.json`) is not the repository's: only a path is followed.
            extends: extended
                .into_iter()
                .filter(|path| path.starts_with(CURRENT))
                .map(|path| {
                    let path = if path.ends_with(CONFIGURATION_EXTENSION) {
                        path
                    } else {
                        format!("{path}{CONFIGURATION_EXTENSION}")
                    };
                    relative(base, RepositoryPath(&path))
                })
                .collect(),
            base_url,
            ..Declared::default()
        })
    }
}

/// Read the project configuration into the domain's declaration: its references, as the configuration files they point at.
pub(super) fn read(manifest: &ManifestFile) -> Result<Declared, ManifestReadingError> {
    Declared::try_from(ManifestSchema { manifest })
}
