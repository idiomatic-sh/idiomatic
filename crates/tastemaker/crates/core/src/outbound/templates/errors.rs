//! The `templates` port's failures.

/// Why a template could not be rendered.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TemplatesError {
    /// Its content does not parse or does not render with the context it was given.
    #[error("the template does not render: {reason}")]
    Render {
        /// What the renderer said.
        reason: String,
    },
    /// Its content reads names the context does not hold.
    #[error("the template does not render: {reason}: no value is given for {names}")]
    Unset {
        /// What the renderer said.
        reason: String,
        /// The names, each in backticks, in order.
        names: String,
    },
}
