//! The partial facts a file or project scope evaluates and publishes under. The scope's owner factory,
//! `open_program_owner`, is the derived concept's (`derived::relation_owner`), which alone holds the access it is given.

/// The partial facts a scope evaluates and publishes under: a file's, or the project's.
#[derive(Debug)]
pub enum ScopePartials<'a> {
    /// A file scope.
    File {
        /// The partials of the evaluation.
        evaluation: &'a [ryo_execution_domain::domain::coverage::types::Partial],
        /// The partials of the publication.
        publication: &'a [ryo_execution_domain::domain::coverage::types::Partial],
    },
    /// A project scope.
    Project {
        /// The partials of the evaluation.
        evaluation: &'a ryo_execution_domain::domain::coverage::types::Partials,
        /// The partials of the publication.
        publication: &'a ryo_execution_domain::domain::coverage::types::Partials,
    },
}
