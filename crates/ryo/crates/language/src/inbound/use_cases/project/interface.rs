//! What the `project` use case promises: the manifests of a repository, read once per run. A language
//! implements this and `parse`, and those two are all of ryo it implements. It says which files are its
//! manifests, reads them through its own tools, and answers, for every manifest relation of the catalog, how it
//! reads it or that it is not in the language. The method of each relation is generated from the
//! catalog's one list (`domain/relations/catalog.rs`) and has no default body.

use super::errors::ProjectError;
use super::types::{ManifestName, ManifestReader, ProjectRequest, Resolved};
use crate::domain::answers::types::Support;

/// What the methods of a trait are made of, one for each manifest relation of the catalog.
macro_rules! manifest_methods {
    (
        syntax { $( $syntax:tt )* }
        manifest { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        engine { $( $engine:tt )* }
    ) => {
        $(
            #[doc = stringify!($module)]
            fn $module(&self) -> Support<ManifestReader<ryo_relations::domain::relations::collections::relations::$module::item::$variant, Self::Derived>>;
        )*
    };
}

/// What a language offers for the repository's manifests.
pub trait ProjectUseCase: Send + Sync {
    /// What the language works out when it reads the manifests, besides the bytes it keeps, for its readers to read: its own type, or `()`
    /// when it works out nothing.
    type Derived: Send + Sync + 'static;

    /// The names of the files the language reads as manifests; a language without any answers none.
    fn manifests(&self) -> &'static [ManifestName];

    /// Classify an explicitly requested primary without IO. This selects source versus project semantics,
    /// independently of whether a particular metadata relation is implemented.
    fn primary_role(
        &self,
        path: crate::domain::files::types::SourcePath<'_>,
    ) -> crate::domain::files::types::PrimaryFileRole;

    /// Candidate entry paths needed for the demanded project facts, without observing the filesystem.
    fn project_entries(
        &self,
        request: &ProjectRequest,
    ) -> Vec<crate::domain::files::types::FilePath>;

    /// Reads the manifests through the language's own tools and answers what the readers below read their
    /// rows from, and the manifests the tools refused; nothing of it is kept for another run. A manifest that does not
    /// parse is refused and the others are read: it is a finding of the run, as a syntax error is.
    ///
    /// # Errors
    ///
    /// `Manifest` when the tools cannot run at all.
    fn resolve(&self, request: &ProjectRequest) -> Result<Resolved<Self::Derived>, ProjectError>;

    ryo_relations::each_relation!(manifest_methods);
}
