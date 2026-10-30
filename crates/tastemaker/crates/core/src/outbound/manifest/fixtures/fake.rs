//! The fake of the `manifest` port: definitions given by the test, by their text, and locks kept as written.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::domain::definitions::types::{
    Definition, RuleFile, SpecFile, TemplateFile, WorkspaceFile,
};
use crate::domain::files::types::FileText;
use crate::domain::packages::types::{DefinitionKind, Lock, PackageManifest};
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::{DefinitionText, EntryDraft};

/// What the core's tests stand in for `manifest` with: each definition by the text that names it.
#[derive(Debug, Default)]
pub struct ManifestFake {
    /// The `tastemaker.toml` files.
    pub workspaces: BTreeMap<Box<str>, WorkspaceFile>,
    /// The specs.
    pub specs: BTreeMap<Box<str>, SpecFile>,
    /// The rules.
    pub rules: BTreeMap<Box<str>, RuleFile>,
    /// The templates.
    pub templates: BTreeMap<Box<str>, TemplateFile>,
    /// The package manifests.
    pub packages: BTreeMap<Box<str>, PackageManifest>,
    /// The locks written, by the text written for each.
    pub locks: Mutex<BTreeMap<Box<str>, Lock>>,
}

/// The refusal of a definition the fake was not given, naming it.
fn missing(text: &DefinitionText) -> ManifestError {
    ManifestError::Shape {
        reason: format!("the fake holds no definition for `{}`", text.0),
    }
}

impl crate::domain::entries::types::ValueWriter for ManifestFake {
    fn write(
        &self,
        value: &crate::domain::entries::types::Value,
    ) -> crate::domain::entries::types::ValueText {
        // The fake writes a value as the TOML adapter does: it reads definitions, never writes their format its own way.
        crate::domain::entries::types::ValueWriter::write(
            &crate::outbound::manifest::factory::toml(),
            value,
        )
    }
}

impl ManifestPort for ManifestFake {
    fn workspace_file(&self, text: &DefinitionText) -> Result<WorkspaceFile, ManifestError> {
        self.workspaces
            .get(&text.0)
            .cloned()
            .ok_or_else(|| missing(text))
    }

    fn spec(&self, text: &DefinitionText) -> Result<SpecFile, ManifestError> {
        self.specs
            .get(&text.0)
            .cloned()
            .ok_or_else(|| missing(text))
    }

    fn rule(&self, text: &DefinitionText) -> Result<RuleFile, ManifestError> {
        self.rules
            .get(&text.0)
            .cloned()
            .ok_or_else(|| missing(text))
    }

    fn template(&self, text: &DefinitionText) -> Result<TemplateFile, ManifestError> {
        self.templates
            .get(&text.0)
            .cloned()
            .ok_or_else(|| missing(text))
    }

    fn definition(
        &self,
        kind: DefinitionKind,
        text: &DefinitionText,
    ) -> Result<Definition, ManifestError> {
        match kind {
            DefinitionKind::Rule => self.rule(text).map(Definition::Rule),
            DefinitionKind::Spec => self.spec(text).map(Definition::Spec),
            DefinitionKind::Template => self.template(text).map(Definition::Template),
        }
    }

    fn package(&self, text: &DefinitionText) -> Result<PackageManifest, ManifestError> {
        self.packages
            .get(&text.0)
            .cloned()
            .ok_or_else(|| missing(text))
    }

    fn lock(&self, text: &DefinitionText) -> Result<Lock, ManifestError> {
        self.locks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&text.0)
            .cloned()
            .ok_or_else(|| missing(text))
    }

    fn lock_text(&self, lock: &Lock) -> DefinitionText {
        let text: Box<str> = format!("{lock:?}").into();
        drop(
            self.locks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert(text.clone(), lock.clone()),
        );
        DefinitionText(text)
    }

    fn add_entry(&self, file: &FileText, entry: &EntryDraft) -> Result<FileText, ManifestError> {
        Ok(FileText(format!("{}\n{}", file.0, entry.name.0).into()))
    }

    fn rebased(
        &self,
        file: &FileText,
        _: &crate::domain::paths::types::RepoPath,
    ) -> Result<FileText, ManifestError> {
        Ok(file.clone())
    }

    fn scenario_root(
        &self,
        root: &crate::domain::scenario_files::types::ScenarioRoot<'_>,
    ) -> DefinitionText {
        DefinitionText(format!("{root:?}").into())
    }

    fn scenario_wrapper(
        &self,
        wrapper: &crate::domain::scenario_files::types::Wrapper<'_>,
    ) -> DefinitionText {
        DefinitionText(format!("{wrapper:?}").into())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
