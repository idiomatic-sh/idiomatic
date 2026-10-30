//! What the `manifest` port asks of the outside, in the core's own vocabulary: the text of a definition read into the
//! core's types, and a `tastemaker.toml` or lock written.

use super::errors::ManifestError;
use super::types::{DefinitionText, EntryDraft};
use crate::domain::definitions::types::{
    Definition, RuleFile, SpecFile, TemplateFile, WorkspaceFile,
};
use crate::domain::files::types::FileText;
use crate::domain::packages::types::{DefinitionKind, Lock, PackageManifest};

/// What every `manifest` adapter implements; as a [`ValueWriter`](crate::domain::entries::types::ValueWriter) it writes a
/// param value as a path or a template reads it: text as written, anything else as the format writes it.
pub trait ManifestPort: Send + Sync + crate::domain::entries::types::ValueWriter {
    /// A `tastemaker.toml`, root or local.
    ///
    /// # Errors
    ///
    /// `Shape` when the text is not one.
    fn workspace_file(&self, text: &DefinitionText) -> Result<WorkspaceFile, ManifestError>;

    /// A `spec.toml`; its owned entries name no package until the spec's release adopts them.
    ///
    /// # Errors
    ///
    /// `Shape` when the text is not one.
    fn spec(&self, text: &DefinitionText) -> Result<SpecFile, ManifestError>;

    /// A `rule.toml`.
    ///
    /// # Errors
    ///
    /// `Shape` when the text is not one.
    fn rule(&self, text: &DefinitionText) -> Result<RuleFile, ManifestError>;

    /// A `template.toml`, its content inline.
    ///
    /// # Errors
    ///
    /// `Shape` when the text is not one.
    fn template(&self, text: &DefinitionText) -> Result<TemplateFile, ManifestError>;

    /// The definition file of `kind`: a `rule.toml`, a `spec.toml` or a `template.toml`, read as `rule`, `spec` and
    /// `template` read it.
    ///
    /// # Errors
    ///
    /// `Shape` when the text is not one.
    fn definition(
        &self,
        kind: DefinitionKind,
        text: &DefinitionText,
    ) -> Result<Definition, ManifestError>;

    /// A `package.toml`.
    ///
    /// # Errors
    ///
    /// `Shape` when the text is not one.
    fn package(&self, text: &DefinitionText) -> Result<PackageManifest, ManifestError>;

    /// A `tastemaker.lock`.
    ///
    /// # Errors
    ///
    /// `Shape` when the text is not one.
    fn lock(&self, text: &DefinitionText) -> Result<Lock, ManifestError>;

    /// The text of a `tastemaker.lock`.
    fn lock_text(&self, lock: &Lock) -> DefinitionText;

    /// A `tastemaker.toml` as the repository holds it with one entry added, its layout kept.
    ///
    /// # Errors
    ///
    /// `Shape` when the text is not TOML.
    fn add_entry(&self, file: &FileText, entry: &EntryDraft) -> Result<FileText, ManifestError>;

    /// A `tastemaker.toml` or a `package.toml` as the repository holds it with each relative `path` of a dependency made absolute,
    /// resolved from `here`, the directory it was written in; its layout kept.
    ///
    /// # Errors
    ///
    /// `Shape` when the text is not TOML.
    fn rebased(
        &self,
        file: &FileText,
        here: &crate::domain::paths::types::RepoPath,
    ) -> Result<FileText, ManifestError>;

    /// The root `tastemaker.toml` a scenario without its own is placed by: the whole project, open, depending on the
    /// definition's package, with one entry naming the definition and holding the scenario's keys.
    fn scenario_root(
        &self,
        root: &crate::domain::scenario_files::types::ScenarioRoot<'_>,
    ) -> DefinitionText;

    /// The `spec.toml` of the wrapper an owned definition is proved through: one entry placing it by its name with the
    /// scenario's keys, open among the scenario's other files.
    fn scenario_wrapper(
        &self,
        wrapper: &crate::domain::scenario_files::types::Wrapper<'_>,
    ) -> DefinitionText;
}
