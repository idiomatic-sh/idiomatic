//! The `project` use case here is the language without manifest relations: it has no manifest and answers
//! every manifest relation as not in it.

use super::errors::ProjectError;
use super::interface::ProjectUseCase;
use super::types::{ManifestName, ProjectRequest, Resolved};

/// A language without manifest relations.
#[derive(Clone, Copy, Debug)]
pub struct ProjectUseCaseImpl;

impl ProjectUseCase for ProjectUseCaseImpl {
    type Derived = ();

    fn manifests(&self) -> &'static [ManifestName] {
        &[]
    }

    fn primary_role(
        &self,
        _path: crate::domain::files::types::SourcePath<'_>,
    ) -> crate::domain::files::types::PrimaryFileRole {
        crate::domain::files::service::primary_role_from_manifests(
            _path,
            self.manifests().iter().map(|manifest| manifest.0),
        )
    }

    fn project_entries(
        &self,
        _request: &ProjectRequest,
    ) -> Vec<crate::domain::files::types::FilePath> {
        Vec::new()
    }

    fn resolve(&self, request: &ProjectRequest) -> Result<Resolved, ProjectError> {
        Ok(Resolved {
            observed_entries: request.observed_entries.clone(),
            refused: Vec::new(),
            unlisted: Vec::new(),
            consulted: Vec::new(),
            derived: (),
        })
    }

    ryo_relations::each_relation!(absent_manifests);
}
