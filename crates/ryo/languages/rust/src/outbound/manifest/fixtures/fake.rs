//! The fake of the `manifest` port: what the language's tests stand in for it with.

use super::super::errors::ManifestError;
use super::super::interface::ManifestPort;
use crate::domain::paths::types::FileText;
use crate::domain::paths::types::Section;
use crate::domain::targets::types::Config;
use crate::domain::targets::types::Manifest;
use crate::domain::targets::types::PackageName;
use crate::domain::targets::types::PackageSection;
use crate::domain::targets::types::WrittenPath;
use crate::domain::workspaces::types::Reason;

/// What the language's tests stand in for `manifest` with: it reads a package by its name and a configuration by its `target-dir`,
/// and refuses text that declares nothing.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ManifestFake;

/// The text between the quotes after `key = ` on some line of `text`.
fn quoted<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(" = \""))
        .and_then(|rest| rest.split('"').next())
}

impl ManifestPort for ManifestFake {
    fn read(&self, text: &FileText) -> Result<Manifest, ManifestError> {
        let text: &str = &text.0;
        let Some(name) = quoted(text, "name") else {
            return Err(ManifestError::Unreadable {
                reason: Reason("the fake reads only a package".into()),
            });
        };
        Ok(Manifest {
            package: Section::Present(PackageSection {
                name: PackageName(name.into()),
                ..PackageSection::default()
            }),
            ..Manifest::default()
        })
    }

    fn configuration(&self, text: &FileText) -> Result<Config, ManifestError> {
        let text: &str = &text.0;
        Ok(Config {
            target_dir: quoted(text, "target-dir").map_or(Section::Absent, |directory| {
                Section::Present(WrittenPath(directory.into()))
            }),
        })
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
