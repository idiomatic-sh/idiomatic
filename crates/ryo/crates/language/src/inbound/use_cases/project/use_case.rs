//! The `project` use case here is the language without manifest relations: it has no manifest and answers
//! every manifest relation as not in it.

use super::errors::ProjectError;
use super::interface::ProjectUseCase;
use super::types::{ProjectRequest, Resolved};
use crate::domain::files::types::ManifestName;

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
            self.manifests().iter().copied(),
        )
    }

    fn project_entries(
        &self,
        _request: &ProjectRequest,
        _entries: &mut crate::domain::files::types::ProjectEntries,
    ) {
    }

    fn resolve(&self, request: &ProjectRequest) -> Result<Resolved, ProjectError> {
        Ok(Resolved {
            observed_entries: crate::domain::project_context::service::receipt(
                &request.observed_entries,
            ),
            refused: Vec::new(),
            unlisted: Vec::new(),
            consulted: Vec::new(),
            derived: (),
        })
    }

    ryo_relations::each_relation!(absent_manifests);
}
