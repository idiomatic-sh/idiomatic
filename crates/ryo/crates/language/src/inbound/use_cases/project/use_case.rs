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

    fn resolve(&self, _request: &ProjectRequest) -> Result<Resolved, ProjectError> {
        Ok(Resolved {
            refused: Vec::new(),
            unlisted: Vec::new(),
            consulted: Vec::new(),
            derived: (),
        })
    }

    crate::each_relation!(crate::absent_manifests);
}
