//! The `walk` use case here is the language that reads nothing: it walks no file. The contract runs against it and against the
//! fixture language that walks one, and a real language replaces it with its own.

use super::errors::WalkError;
use super::interface::WalkUseCase;
use super::types::{FileVisit, RelationNeeds, WalkEngine, WalkRequest};
use crate::domain::fillers::types::{Fillers, WalkVocabulary};
use crate::domain::node_table::service::builder;
use crate::domain::node_table::types::{NodeTable, NodeTableBuilder};
use crate::domain::semantics::types::{RenderOutcome, RenderRequest};
use crate::domain::diagnostics::types::SyntaxCompleteness;
use crate::domain::facts::types::FileCompleteness;
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{Bytes, Count, KindBits};

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

    fn completeness(_file: &()) -> FileCompleteness {
        FileCompleteness { syntax: SyntaxCompleteness::Complete, lexical: SyntaxCompleteness::Complete }
    }

    fn fills_are_complete(_file: &mut ()) -> bool {
        false
    }

    fn notes(_file: &mut (), _table: &NodeTable, _source: ByteSlice<'_>, _out: &mut Bytes) {}

    fn spells(_kind: KindBits) -> bool {
        false
    }
}

/// Every `WalkUseCase` is a `WalkEngine`: the host walks it with a table and buffers the language's own types stay inside.
impl<W: WalkUseCase> WalkEngine for W {
    fn kind_count(&self) -> Count {
        W::KIND_COUNT
    }

    fn vocabulary(&self) -> WalkVocabulary {
        <W as Fillers>::vocabulary()
    }

    fn buffers(&self) -> Box<dyn std::any::Any + Send> {
        Box::new(W::Scratch::default())
    }

    fn walk_file(
        &self,
        request: WalkRequest<'_>,
        needs: RelationNeeds<'_>,
        table: &mut NodeTable,
        buffers: &mut (dyn std::any::Any + Send),
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError> {
        let Some(scratch) = buffers.downcast_mut::<W::Scratch>() else {
            return Err(WalkError::Library);
        };
        let writer = &mut builder(table, W::KIND_COUNT);
        self.walk(request, needs, writer, scratch, visit)
    }
}
