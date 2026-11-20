//! The `walk` use case here is the language that reads nothing: it walks no file. The contract runs against it and against the
//! fixture language that walks one, and a real language replaces it with its own.

use super::errors::WalkError;
use super::interface::WalkUseCase;
use super::types::{FileVisit, RelationNeeds, WalkRequest};
use crate::domain::fillers::types::Fillers;
use crate::domain::node_table::types::NodeTableBuilder;
use crate::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_relations::domain::relations::types::Count;

/// A language that walks no file.
///
/// # Examples
///
/// ```
/// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
///
/// let _language = WalkUseCaseImpl::new();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct WalkUseCaseImpl;

impl WalkUseCaseImpl {
    /// The language that reads nothing; it holds no port.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
    ///
    /// let _language = WalkUseCaseImpl::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl WalkUseCase for WalkUseCaseImpl {
    type Scratch = ();
    const KIND_COUNT: Count = Count(0);

    fn walk(
        &self,
        _request: WalkRequest<'_>,
        _needs: RelationNeeds<'_>,
        _builder: &mut NodeTableBuilder<'_>,
        _scratch: &mut (),
        _visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError> {
        Err(WalkError::Unsupported)
    }
}

impl Fillers for WalkUseCaseImpl {
    type File<'f> = ();

    ryo_relations::each_relation!(crate::no_fillers);

    fn render(_file: &mut (), _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }
}
