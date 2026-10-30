//! The `list` use case: the definitions of the workspace's packages, read from the packages themselves.

use std::sync::Arc;

use super::errors::ListError;
use super::interface::ListUseCase;
use super::types::{Kinds, ListReport, ListRequest, Listed, ListedKind, SearchTerm};
use crate::domain::definitions::constants::WORKSPACE_FILE;
use crate::domain::definitions::service::description;
use crate::domain::definitions::types::Role;
use crate::domain::entries::types::Description;
use crate::domain::packages::constants::PACKAGE_FILE;
use crate::domain::packages::service::{kind_directory, kind_file, latest};
use crate::domain::packages::types::{
    DefinitionKind, DefinitionName, PackageName, PackageRoot, SourcePath, VersionText,
};
use crate::domain::paths::service::join;
use crate::domain::paths::types::PathText;
use crate::outbound::analysis::interface::AnalysisPort;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::DefinitionText;
use crate::outbound::packages::interface::PackagesPort;
use crate::outbound::packages::types::PackageFile;
use crate::outbound::repository::interface::RepositoryPort;

/// `list`, over the ports the edge injects.
pub struct ListUseCaseImpl {
    /// Reads and writes the workspace's files.
    repository: Arc<dyn RepositoryPort>,
    /// Parses and edits the manifests and the lock.
    manifest: Arc<dyn ManifestPort>,
    /// Locates the packages and reads their files.
    packages: Arc<dyn PackagesPort>,
    /// Answers the analysis the definitions ask for.
    analysis: Arc<dyn AnalysisPort>,
}

impl std::fmt::Debug for ListUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl ListUseCaseImpl {
    /// `list` over the ports the edge injects.
    #[must_use]
    pub fn new(
        repository: Arc<dyn RepositoryPort>,
        manifest: Arc<dyn ManifestPort>,
        packages: Arc<dyn PackagesPort>,
        analysis: Arc<dyn AnalysisPort>,
    ) -> Self {
        Self {
            repository,
            manifest,
            packages,
            analysis,
        }
    }

    /// The relations or operators of ryo's catalog whose name or summary holds the term: no workspace is read.
    fn catalogued(&self, kind: ListedKind, term: &SearchTerm) -> Result<ListReport, ListError> {
        let entries = if kind == ListedKind::Relation {
            self.analysis.relations()?
        } else {
            self.analysis.operators()?
        }
        .0;
        let listed = entries
            .into_iter()
            .filter(|entry| {
                term.0.is_empty()
                    || entry.name.0.contains(&*term.0)
                    || entry.summary.0.contains(&*term.0)
            })
            .map(|entry| Listed {
                kind,
                package: PackageName(Box::default()),
                name: DefinitionName(entry.name.0),
                version: VersionText(Box::default()),
                description: Description(entry.summary.0),
            })
            .collect::<Vec<_>>();
        // A term nothing holds is refused, so a misspelt kind or name is said rather than listed as nothing.
        if listed.is_empty() && !term.0.is_empty() {
            return Err(ListError::NoMatch {
                term: term.0.to_string(),
            });
        }
        Ok(ListReport { listed })
    }

    /// What one definition is, read from its latest version.
    ///
    /// # Errors
    ///
    /// `Packages` when the definition's file cannot be read; `Manifest` when it does not read as its kind.
    fn description(
        &self,
        directory: &PackageRoot,
        kind: DefinitionKind,
        file: &PackageFile,
    ) -> Result<Description, ListError> {
        let text = self.packages.read(directory, file)?;
        self.manifest
            .definition(kind, &DefinitionText(text.0))
            .map(|definition| description(&definition))
            .map_err(|source| ListError::Manifest {
                file: file.0.to_string(),
                source,
            })
    }
}

impl ListUseCase for ListUseCaseImpl {
    fn list(&self, request: ListRequest) -> Result<ListReport, ListError> {
        match request.kinds {
            Kinds::Relations => return self.catalogued(ListedKind::Relation, &request.term),
            Kinds::Operators => return self.catalogued(ListedKind::Operator, &request.term),
            Kinds::All | Kinds::Only(_) => {}
        }
        let found = self.repository.find(&request.start)?;
        let repository = &*found;
        let file = join(found.workspace(), PathText(WORKSPACE_FILE)).unwrap_or_default();
        let text = repository.read(&file)?;
        let workspace = self
            .manifest
            .workspace_file(&DefinitionText(text.0))
            .map_err(|source| ListError::Manifest {
                file: file.0.to_string(),
                source,
            })?;
        let Role::Root(section) = workspace.role else {
            return Ok(ListReport { listed: Vec::new() });
        };
        let from = PackageRoot(
            std::path::Path::new(&*found.root().0)
                .join(&*found.workspace().0)
                .display()
                .to_string()
                .into(),
        );
        let mut listed = Vec::new();
        for dependency in &section.dependencies {
            let located = self
                .packages
                .locate(dependency, &from, &SourcePath(Box::default()))?;
            // A dependency whose package file is missing or unreadable is refused.
            self.packages
                .offers(&located.directory, &PackageFile(PACKAGE_FILE.into()))?;
            let kinds = match request.kinds {
                Kinds::All => vec![
                    DefinitionKind::Rule,
                    DefinitionKind::Spec,
                    DefinitionKind::Template,
                ],
                Kinds::Only(kind) => vec![kind],
                Kinds::Relations | Kinds::Operators => Vec::new(),
            };
            for kind in kinds {
                let directory = kind_directory(kind).0;
                for name in self
                    .packages
                    .versions(&located.directory, &PackageFile(directory.into()))?
                {
                    let within = format!("{directory}/{}", name.0);
                    let versions = self
                        .packages
                        .versions(&located.directory, &PackageFile(within.clone().into()))?;
                    let Some(first) = versions.first() else {
                        continue;
                    };
                    let version = latest(first, versions.iter());
                    let description = self.description(
                        &located.directory,
                        kind,
                        &PackageFile(format!("{within}/{}/{}", version.0, kind_file(kind)).into()),
                    )?;
                    let term = &*request.term.0;
                    if !term.is_empty() && !name.0.contains(term) && !description.0.contains(term) {
                        continue;
                    }
                    listed.push(Listed {
                        kind: ListedKind::Definition(kind),
                        package: dependency.package.clone(),
                        name: DefinitionName(name.0.clone()),
                        version: VersionText(version.0.clone()),
                        description,
                    });
                }
            }
        }
        listed.sort();
        // A term nothing holds is refused, so a misspelt kind or name is said rather than listed as nothing.
        if listed.is_empty() && !request.term.0.is_empty() {
            return Err(ListError::NoMatch {
                term: request.term.0.to_string(),
            });
        }
        Ok(ListReport { listed })
    }
}
