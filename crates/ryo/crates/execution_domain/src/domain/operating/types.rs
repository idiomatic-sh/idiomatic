//! The types of the `operating` concept: what an operator reads, writes and fails with. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operating::errors::OperatingError;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_values::domain::files::types::SourceView;

/// An application-owned, file-scoped semantic demand port. Operators borrow it;
/// they never own a provider session or its cache.
pub trait SemanticLookup: std::fmt::Debug {
    /// Resolve demanded keys in order; the current application deduplicates each batch.
    ///
    /// # Errors
    ///
    /// The implementation's [`OperatingError`]: a request it does not admit, a language without semantics, or an
    /// answer batch whose length differs from the requests'.
    fn render_batch(
        &self,
        requests: &[RenderRequest],
        source: SourceView<'_>,
    ) -> Result<Vec<RenderOutcome>, OperatingError>;
}

/// One of the inputs of a step, by its place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputPlace(pub usize);

/// Whether a comparison asks of the kind of an element something fixed, which the sets of kinds decide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KindNaming {
    /// It names a kind.
    Names,
    /// It does not.
    Silent,
}

/// Whether the steps run before the files were read, or after: after, the text of the workers is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterTheFiles {
    /// While the files are read.
    Before,
    /// After the files were read.
    After,
}
