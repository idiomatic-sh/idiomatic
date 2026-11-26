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
use crate::domain::files::types::SourcePath;
use crate::inbound::use_cases::parse::named::{FileRelation, SourceWords};
use crate::inbound::use_cases::parse::types::{ParseConfigurationIdentity, Recognition, SkippedDirectory};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::catalog::types::OptionalCapability;
use ryo_relations::domain::relations::service::{all, optional_capability, provenance_of};
use ryo_relations::domain::relations::types::{Bytes, Count, KindBits, Provenance, Relation};
use ryo_relations::domain::terms::types::UnknownName;

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

    fn recognizes(&self, _path: SourcePath<'_>, _explicit: bool) -> Recognition {
        Recognition::Declines
    }

    fn skips(&self) -> &'static [SkippedDirectory] {
        &[]
    }

    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        ParseConfigurationIdentity("")
    }

    fn capabilities(&self) -> &'static [OptionalCapability] {
        &[]
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
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

    fn absent(_relation: Relation) -> Option<AbsenceCause> {
        Some(AbsenceCause::NoSuchConstruct)
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

    fn recognizes(&self, path: SourcePath<'_>, explicit: bool) -> Recognition {
        WalkUseCase::recognizes(self, path, explicit)
    }

    fn spells(&self, kind: KindBits) -> bool {
        <W as Fillers>::spells(kind)
    }

    fn skips(&self) -> &'static [SkippedDirectory] {
        WalkUseCase::skips(self)
    }

    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        WalkUseCase::configuration_identity(self)
    }

    fn capabilities(&self) -> Vec<OptionalCapability> {
        let mut promised: Vec<OptionalCapability> = all()
            .iter()
            .filter(|relation| provenance_of(**relation) == Provenance::Trees)
            .filter(|relation| <W as Fillers>::absent(**relation) != Some(AbsenceCause::NotYetRead))
            .filter_map(|relation| optional_capability(*relation))
            .collect();
        promised.extend_from_slice(WalkUseCase::capabilities(self));
        promised
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        WalkUseCase::unknown_reasons(self)
    }

    fn source_words(&self) -> Option<SourceWords> {
        WalkUseCase::source_words(self)
    }

    fn includes_expansions(&self, relation: Relation) -> bool {
        WalkUseCase::includes_expansions(self, relation)
    }

    fn named_declarations(&self) -> &'static [FileRelation] {
        WalkUseCase::named_declarations(self)
    }

    fn uses_project(&self) -> bool {
        WalkUseCase::uses_project(self)
    }

    fn absence_of(&self, relation: Relation) -> Option<AbsenceCause> {
        <W as Fillers>::absent(relation)
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
