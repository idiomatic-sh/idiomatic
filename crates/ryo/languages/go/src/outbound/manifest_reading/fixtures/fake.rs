//! The fake of the `manifest_reading` port: what the language's tests stand in for it with.

use super::super::errors::ManifestReadingError;
use super::super::interface::ManifestReadingPort;
use crate::domain::modules::types::{Declared, Manifestness, ModulePath};
use ryo::languages::ManifestText;

/// What the language's tests stand in for `project` with: a manifest declares the module its first line names, and a manifest with a
/// closing parenthesis on a line of its own is refused.
pub(crate) struct ManifestReadingFake;

impl ManifestReadingPort for ManifestReadingFake {
    fn declared(
        &self,
        kind: Manifestness,
        text: &ManifestText,
    ) -> Result<Declared, ManifestReadingError> {
        if text.lines().any(|line| line.trim() == ")") {
            return Err(ManifestReadingError::Unopened { line: 1 });
        }
        let module = text
            .lines()
            .find_map(|line| line.strip_prefix("module "))
            .map(|path| ModulePath(path.trim().into()));
        if kind == Manifestness::Module && module.is_none() {
            return Err(ManifestReadingError::NoModule);
        }
        Ok(Declared {
            module,
            ..Declared::default()
        })
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
