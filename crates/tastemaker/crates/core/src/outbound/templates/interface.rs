//! What the `templates` port asks of the outside, in the core's own vocabulary: a template's content rendered, and the
//! names it reads.

use super::errors::TemplatesError;
use crate::domain::contexts::types::{ContextKeys, RenderContext};
use crate::domain::definitions::types::TemplateText;
use crate::domain::files::types::FileText;

/// What every `templates` adapter implements.
pub trait TemplatesPort: Send + Sync {
    /// `template` rendered with `context`: a name it reads that the context does not hold is an error, unless the
    /// template gives it a `default`.
    ///
    /// # Errors
    ///
    /// `Render` when it does not parse or does not render.
    fn render(
        &self,
        template: &TemplateText,
        context: &RenderContext,
    ) -> Result<FileText, TemplatesError>;

    /// The names `template` may read from its context, whatever its control flow: a nested lookup is each of its parts,
    /// so `{{ captures.lib }}` reads `captures` and `lib`.
    ///
    /// # Errors
    ///
    /// `Render` when it does not parse.
    fn reads(&self, template: &TemplateText) -> Result<ContextKeys, TemplatesError>;
}
