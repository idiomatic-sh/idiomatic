//! The partial facts a scope evaluates and publishes under.

/// The partial facts a scope evaluates and publishes under: a file's, or the project's.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::derived::relation_owner::types::ScopePartials;
///
/// let partials = ScopePartials::File { evaluation: &[], publication: &[] };
/// assert!(matches!(partials, ScopePartials::File { .. }));
/// ```
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
