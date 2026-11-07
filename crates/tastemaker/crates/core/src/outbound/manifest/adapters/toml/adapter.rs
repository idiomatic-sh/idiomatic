//! The manifest port, implemented with `toml` and `toml_edit`: `toml` reads a definition into its schema, the schema is
//! checked into the core's type, and `toml_edit` writes a `tastemaker.toml`, keeping its layout. The reading and the
//! writing are the wire format's, in `schemas.rs`.

use super::schemas;
use crate::domain::definitions::types::{
    Definition, RuleFile, SpecFile, TemplateFile, WorkspaceFile,
};
use crate::domain::files::types::FileText;
use crate::domain::packages::types::{DefinitionKind, Lock, PackageManifest};
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::{DefinitionText, EntryDraft};

/// The TOML manifest adapter.
#[derive(Debug)]
pub(crate) struct ManifestTomlAdapter;

impl crate::domain::entries::types::ValueWriter for ManifestTomlAdapter {
    fn write(
        &self,
        value: &crate::domain::entries::types::Value,
    ) -> crate::domain::entries::types::ValueText {
        schemas::value_text(value)
    }
}

impl ManifestPort for ManifestTomlAdapter {
    fn workspace_file(&self, text: &DefinitionText) -> Result<WorkspaceFile, ManifestError> {
        schemas::workspace_file(text)
    }

    fn spec(&self, text: &DefinitionText) -> Result<SpecFile, ManifestError> {
        schemas::spec(text)
    }

    fn rule(&self, text: &DefinitionText) -> Result<RuleFile, ManifestError> {
        schemas::rule(text)
    }

    fn template(&self, text: &DefinitionText) -> Result<TemplateFile, ManifestError> {
        schemas::template(text)
    }

    fn definition(
        &self,
        kind: DefinitionKind,
        text: &DefinitionText,
    ) -> Result<Definition, ManifestError> {
        match kind {
            DefinitionKind::Rule => schemas::rule(text).map(Definition::Rule),
            DefinitionKind::Spec => schemas::spec(text).map(Definition::Spec),
            DefinitionKind::Template => schemas::template(text).map(Definition::Template),
        }
    }

    fn package(&self, text: &DefinitionText) -> Result<PackageManifest, ManifestError> {
        schemas::package(text)
    }

    fn lock(&self, text: &DefinitionText) -> Result<Lock, ManifestError> {
        schemas::lock(text)
    }

    fn lock_text(&self, lock: &Lock) -> DefinitionText {
        schemas::lock_text(lock)
    }

    fn scenario_root(
        &self,
        root: &crate::domain::scenario_files::types::ScenarioRoot<'_>,
    ) -> DefinitionText {
        schemas::scenario_root(root)
    }

    fn scenario_wrapper(
        &self,
        wrapper: &crate::domain::scenario_files::types::Wrapper<'_>,
    ) -> DefinitionText {
        schemas::scenario_wrapper(wrapper)
    }

    fn add_entry(&self, file: &FileText, entry: &EntryDraft) -> Result<FileText, ManifestError> {
        schemas::add_entry(file, entry)
    }

    fn rebased(
        &self,
        file: &FileText,
        here: &crate::domain::paths::types::RepoPath,
    ) -> Result<FileText, ManifestError> {
        schemas::rebased(file, here)
    }
}
