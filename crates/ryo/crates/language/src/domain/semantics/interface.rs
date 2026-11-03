//! An owned semantic session for one physical file generation.

use super::types::{RenderError, RenderOutcome, RenderRequest};
use crate::domain::symbols::types::ByteSlice;

/// A language's private rendering state. The common application owns its lifetime and cache.
pub trait SemanticSession: Send {
    /// Answer a batch in request order, atomically. A successful answer has exactly one result per request.
    ///
    /// # Errors
    ///
    /// A stale or foreign node refuses the file; a partial vector is never returned.
    fn render_batch(
        &mut self,
        source: ByteSlice<'_>,
        requests: &[RenderRequest],
    ) -> Result<Vec<RenderOutcome>, RenderError>;
}
