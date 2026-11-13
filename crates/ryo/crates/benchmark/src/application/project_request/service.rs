//! One run of a corpus project over its explicit files, with its own state directory and its context sealed by its mode.
use super::types::ProjectCorpus;
use crate::outbound::context::interface::ContextPort;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    ContextSeal, FileMetadataDeclaration, InputDeclaration, RunRequest, StagedSources,
};
use ryo_benchmark_contracts::domain::workloads::types::ConfigurationOverride;
use std::path::{Path, PathBuf};

/// The request one run of the project at `root` sends both contestants.
///
/// # Errors
/// Refuses a rule configuration that does not resolve, and a listed-disk context that cannot be sealed.
pub fn request(
    context: &dyn ContextPort,
    corpus: &ProjectCorpus<'_>,
    root: &Path,
    files: &[ryo::linter::RelativePath],
    tracked: &[PathBuf],
    state: PathBuf,
) -> Result<RunRequest, Error> {
    Ok(RunRequest {
        performance: Default::default(),
        selected_rules: corpus.cohort.clone(),
        rule_observations: InputDeclaration::Undeclared,
        rule_context: crate::domain::rule_context::service::resolve(
            corpus.rule_context,
            &ConfigurationOverride::Inherited,
        )?,
        project_search_roots: corpus.search_roots.clone(),
        given_project_inventory: Default::default(),
        staged_sources: StagedSources::Unlisted,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context_manifest: seal_context(context, corpus.context_mode, root, files, tracked)?,
        root: root.to_owned(),
        files: files.to_vec(),
        state,
    })
}

/// Select the declared generic corpus mode and delegate all observation to the context boundary.
fn seal_context(
    context: &dyn ContextPort,
    mode: ProjectContextMode,
    root: &Path,
    files: &[ryo::linter::RelativePath],
    tracked: &[PathBuf],
) -> Result<ContextSeal, Error> {
    match mode {
        ProjectContextMode::Isolated => Ok(ContextSeal::Unsealed),
        ProjectContextMode::ListedDisk => context
            .seal_existing(
                root,
                files,
                crate::outbound::context::types::CorpusFiles(tracked),
            )
            .map(ContextSeal::Sealed),
    }
}
