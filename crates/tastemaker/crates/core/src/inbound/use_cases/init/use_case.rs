//! Initialize an existing project through the repository port.
use super::errors::InitError;
use super::interface::InitUseCase;
use super::types::{InitReport, InitRequest};
use crate::domain::definitions::constants::WORKSPACE_FILE;
use crate::domain::definitions::types::Role;
use crate::domain::files::types::FileText;
use crate::domain::packages::constants::{INIT_WORKSPACE_FILE, INITIAL_FILES};
use crate::domain::paths::types::RepoPath;
use crate::domain::writing::types::Writing;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::DefinitionText;
use crate::outbound::repository::interface::{Repository, RepositoryPort};
use crate::outbound::repository::types::Presence;
use std::sync::Arc;
/// Initialization over the repository supplied by the edge.
pub struct InitUseCaseImpl {
    /// Reads and writes the workspace's files.
    repository: Arc<dyn RepositoryPort>,
    /// Reads the `tastemaker.toml` of a directory below the project, to tell the workspace of its own from a local file.
    manifest: Arc<dyn ManifestPort>,
}

impl std::fmt::Debug for InitUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
impl InitUseCaseImpl {
    /// Supplies the repository used for initialization, and the reader of the workspace files below it.
    #[must_use]
    pub fn new(repository: Arc<dyn RepositoryPort>, manifest: Arc<dyn ManifestPort>) -> Self {
        Self {
            repository,
            manifest,
        }
    }

    /// The files of `walked` the project's workspace governs: those outside every nested workspace, a directory below the
    /// project whose own `tastemaker.toml` is a root, whose tree the project's workspace does not govern. A file that does
    /// not read is no workspace, and its directory is governed.
    fn governed(&self, target: &dyn Repository, walked: &[RepoPath]) -> Vec<RepoPath> {
        let nested = walked
            .iter()
            .filter_map(|file| {
                file.0
                    .strip_suffix(WORKSPACE_FILE)
                    .and_then(|directory| directory.strip_suffix('/'))
                    .map(|directory| (directory, file))
            })
            .filter(|(_, file)| {
                let Ok(text) = target.read(file) else {
                    return false;
                };
                let Ok(read) = self.manifest.workspace_file(&DefinitionText(text.0)) else {
                    return false;
                };
                matches!(read.role, Role::Root(_))
            })
            .map(|(directory, _)| format!("{directory}/"))
            .collect::<Vec<_>>();
        walked
            .iter()
            .filter(|file| !nested.iter().any(|prefix| file.0.starts_with(prefix)))
            .cloned()
            .collect()
    }
}
impl InitUseCase for InitUseCaseImpl {
    fn init(&self, request: InitRequest) -> Result<InitReport, InitError> {
        // Every other command reads a workspace inside a git repository, so one outside any is refused before a write.
        let _repository = self.repository.repository(&request.target)?;
        let target = self.repository.open(&request.target);
        let mut files = INITIAL_FILES
            .iter()
            .map(|(file, text)| (*file, FileText((*text).into())))
            .collect::<Vec<_>>();
        // The workspace file is written last, closed over every path the project holds before `init` writes its own.
        let walked = target.walk(&RepoPath::default())?;
        files.push((
            INIT_WORKSPACE_FILE,
            super::service::workspace_text(&self.governed(&*target, &walked.files)),
        ));
        for (file, _) in &files {
            if target.exists(&RepoPath((*file).into())) == Presence::Present {
                return Err(InitError::Exists {
                    file: (*file).to_owned(),
                });
            }
        }
        let mut written = Vec::new();
        for (file, text) in files {
            let path = RepoPath(file.into());
            if request.writing == Writing::Write {
                target.write(&path, &text)?;
            }
            written.push(path);
        }
        Ok(InitReport { written })
    }
}
