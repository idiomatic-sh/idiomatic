//! The `create` use case: a definition's skeleton written into its package.

use std::sync::Arc;

use super::errors::CreateError;
use super::interface::CreateUseCase;
use super::types::{CreateReport, CreateRequest, Destination, Extension};
use crate::domain::definitions::constants::WORKSPACE_FILE;
use crate::domain::definitions::types::Role;
use crate::domain::files::types::MachinePath;
use crate::domain::packages::constants::{FIRST_VERSION, SCENARIO_EXTENSION};
use crate::domain::packages::service::{kind_directory, kind_file, kind_skeleton, kind_word};
use crate::domain::packages::types::{PackageRoot, PackageSource, SourcePath};
use crate::domain::paths::service::join;
use crate::domain::paths::types::PathText;
use crate::domain::paths::types::RepoPath;
use crate::domain::writing::types::Writing;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::DefinitionText;
use crate::outbound::packages::interface::PackagesPort;
use crate::outbound::repository::interface::RepositoryPort;
use crate::outbound::repository::types::Presence;

/// `create`, over the ports the edge injects.
pub struct CreateUseCaseImpl {
    /// Reads and writes the workspace's files.
    repository: Arc<dyn RepositoryPort>,
    /// Parses and edits the manifests and the lock.
    manifest: Arc<dyn ManifestPort>,
    /// Locates the packages and reads their files.
    packages: Arc<dyn PackagesPort>,
}

impl std::fmt::Debug for CreateUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl CreateUseCaseImpl {
    /// `create` over the ports the edge injects.
    #[must_use]
    pub fn new(
        repository: Arc<dyn RepositoryPort>,
        manifest: Arc<dyn ManifestPort>,
        packages: Arc<dyn PackagesPort>,
    ) -> Self {
        Self {
            repository,
            manifest,
            packages,
        }
    }

    /// The package the request writes into.
    fn package(&self, request: &CreateRequest) -> Result<MachinePath, CreateError> {
        if let Destination::Package(directory) = &request.into {
            return Ok(directory.clone());
        }
        let found = self.repository.find(&request.start)?;
        let repository = &*found;
        let file = join(found.workspace(), PathText(WORKSPACE_FILE)).unwrap_or_default();
        let text = repository.read(&file)?;
        let workspace = self
            .manifest
            .workspace_file(&DefinitionText(text.0))
            .map_err(|source| CreateError::Manifest {
                file: file.0.to_string(),
                source,
            })?;
        let Role::Root(section) = workspace.role else {
            return Err(CreateError::NoPackage);
        };
        let from = PackageRoot(
            std::path::Path::new(&*found.root().0)
                .join(&*found.workspace().0)
                .display()
                .to_string()
                .into(),
        );
        let dependency = section
            .dependencies
            .iter()
            .find(|dependency| matches!(dependency.source, PackageSource::Path(_)))
            .ok_or(CreateError::NoPackage)?;
        let located = self
            .packages
            .locate(dependency, &from, &SourcePath(Box::default()))?;
        Ok(MachinePath(located.directory.0))
    }
}

impl CreateUseCase for CreateUseCaseImpl {
    fn create(&self, request: CreateRequest) -> Result<CreateReport, CreateError> {
        let package = self.package(&request)?;
        let directory = self.repository.open(&package);
        let file = RepoPath(
            format!(
                "{}/{}/{FIRST_VERSION}/{}",
                kind_directory(request.kind),
                request.name.0,
                kind_file(request.kind)
            )
            .into(),
        );
        if directory.exists(&file) == Presence::Present {
            return Err(CreateError::Exists {
                file: file.0.to_string(),
            });
        }
        let mut skeleton = kind_skeleton(request.kind);
        if skeleton.0.contains(SCENARIO_EXTENSION) {
            // The skeleton's scenario files end in the extension the person names: none is guessed or defaulted.
            let Extension::Named(extension) = &request.extension else {
                return Err(CreateError::NoExtension {
                    kind: kind_word(request.kind).0.to_owned(),
                    name: request.name.0.to_string(),
                });
            };
            skeleton.0 = skeleton.0.replace(SCENARIO_EXTENSION, &extension.0).into();
        }
        if request.writing == Writing::Write {
            directory.write(&file, &skeleton)?;
        }
        Ok(CreateReport {
            package,
            file,
            kind: request.kind,
            name: request.name,
        })
    }
}
