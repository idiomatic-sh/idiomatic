//! The test data of `linter lint` that fixes the language of words: files of words (`*.walk`), which the old engine reads
//! through a parse that makes a sentence of one token for each word, and the new engine through the language crate's fixture
//! walk of the same files; a rule over them compiled from its text; and a linter over that language with the engine port's
//! fake in the new engine's place, in the engines a row says or the ones the environment selects, whose compiler of queries
//! counts the rules it compiles.

use crate::domain::capacity::types::{MemoryBudget, WorkerPool};
use crate::domain::file::types::RelativePath;
use crate::domain::language::types::{Language, LanguageBuild, LanguageName};
use crate::resources::linter::lint::types::{Cache, Corpus, LintRequest, Linter, Workers};
use ryo_execution::outbound::query_language::errors::QueryLanguageError;
use ryo_execution::outbound::query_language::interface::QueryLanguagePort;
use ryo_execution_runtime::outbound::engine::fixtures::fake::EngineFake;
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::fillers::fixtures::Walking;
use ryo_language::domain::semantics::types::SemanticSlot;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_language::inbound::use_cases::parse::errors::ParseError;
use ryo_language::inbound::use_cases::parse::types::{
    KindNames, ParseConfigurationIdentity, ParseEngine, ParseRequest, Recognition, SkippedDirectory,
};
use ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl;
use ryo_operators::spike::EngineMode;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::catalog::types::OptionalCapability;
use ryo_relations::domain::relations::types::{
    Count, FileId, KindBits, Offset, Relation, RowWriter,
};
use ryo_relations::domain::terms::types::{NameText, UnknownName};
use ryo_values::domain::descriptions::types::ValidateReport;
use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::observations::types::BoundObservationInput;
use ryo_values::domain::query_plans::types::Plan;
use ryo_values::domain::program::types::ParsedModules;
use ryo_values::domain::query_tokens::types::QueryBundle;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The rule of the fixture: every word of a file, by its element, its file, its span and its text.
pub(crate) const WORDS: &str = "let words = select id, file, span, text from \"syntax-tree\" where kind = \"word\"; emit words subject file key(id);";

/// The same rule under another text: its digest differs, so its pack is another.
pub(crate) const WORDS_AGAIN: &str = "let found = select id, file, span, text from \"syntax-tree\" where kind = \"word\"; emit found subject file key(id);";

/// The compiler of queries a fixture linter compiles its rules with: the linter's own, counting the rules it is asked to compile.
pub(crate) struct CountingCompiler {
    /// The linter's own compiler.
    inner: Arc<dyn QueryLanguagePort>,
    /// How many rules it was asked to compile.
    pub(crate) compiled: AtomicUsize,
}

impl QueryLanguagePort for CountingCompiler {
    fn validate(
        &self,
        bundle: &QueryBundle,
        hosted: &[HostedRelation],
        observations: &[BoundObservationInput],
    ) -> Result<ValidateReport, QueryLanguageError> {
        self.inner.validate(bundle, hosted, observations)
    }

    fn compile(
        &self,
        bundle: &QueryBundle,
        hosted: &[HostedRelation],
        observations: &[BoundObservationInput],
    ) -> Result<Plan, QueryLanguageError> {
        let _before = self.compiled.fetch_add(1, Ordering::Relaxed);
        self.inner.compile(bundle, hosted, observations)
    }

    fn compile_parsed(
        &self,
        bundle: &QueryBundle,
        modules: &mut ParsedModules,
        hosted: &[HostedRelation],
        observations: &[BoundObservationInput],
    ) -> Result<Plan, QueryLanguageError> {
        let _before = self.compiled.fetch_add(1, Ordering::Relaxed);
        self.inner
            .compile_parsed(bundle, modules, hosted, observations)
    }
}

/// The kinds of the language's trees, in the order of their codes: the kind no element has, the sentence, the word. The
/// fixture walk numbers its sentence 1 and its words 2.
const KINDS: [&str; 3] = ["blank", "sentence", "word"];

/// The old engine's parse of a file of words: a sentence over one token for each word. `short` bytes are left off the end of
/// each word: the difference a row plants between the two engines.
pub(crate) struct WordsParse {
    /// How many bytes each word's token ends early.
    pub(crate) short: u32,
}

impl ParseEngine for WordsParse {
    fn capabilities(&self) -> Vec<OptionalCapability> {
        Vec::new()
    }

    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        ParseConfigurationIdentity("")
    }

    fn begin_semantics(&self, _: &ParseRequest<'_>, _: &Elements, _: &mut SemanticSlot) {}

    fn recognition(&self, request: &ParseRequest<'_>) -> Recognition {
        if request.path.0.ends_with(".walk") {
            Recognition::Reads
        } else {
            Recognition::Declines
        }
    }

    fn skips(&self) -> &'static [SkippedDirectory] {
        &[]
    }

    fn kind_count(&self) -> Count {
        Count(KINDS.len())
    }

    fn kind_names(&self) -> KindNames {
        KindNames(KINDS.map(NameText).to_vec())
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }

    fn reader_kinds(&self) -> Vec<KindBits> {
        Vec::new()
    }

    fn build_file(
        &self,
        request: ParseRequest<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
        _: &[Relation],
    ) -> Result<(), ParseError> {
        if self.recognition(&request) == Recognition::Declines {
            return Err(ParseError::Unsupported);
        }
        let bytes = request.bytes.0;
        let offset = |at: usize| Offset(u32::try_from(at).unwrap_or(u32::MAX));
        {
            let mut builder = table_builder(table, symbols, request.bytes);
            enter_node(&mut builder, KindBits(1), Offset(0));
            let mut start = None;
            for (at, byte) in bytes.iter().chain(std::iter::once(&b' ')).enumerate() {
                match (start, byte.is_ascii_whitespace()) {
                    (None, false) => start = Some(at),
                    (Some(from), true) => {
                        let end = Offset(offset(at).0.saturating_sub(self.short));
                        token(&mut builder, KindBits(2), offset(from), end);
                        start = None;
                    }
                    _ => {}
                }
            }
            leave_node(&mut builder, offset(bytes.len()));
        }
        let mut named = completion(table, symbols, request.bytes, request.file);
        for (code, kind) in KINDS.iter().enumerate() {
            name_kind(
                &mut named,
                KindBits(u16::try_from(code).unwrap_or(u16::MAX)),
                NameText(kind),
            );
        }
        Ok(())
    }

    fn build_file_explicit(
        &self,
        request: ParseRequest<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
        wanted: &[Relation],
    ) -> Result<(), ParseError> {
        self.build_file(request, table, symbols, wanted)
    }

    fn baseline_file(&self, _: &ParseRequest<'_>) -> Result<(), ParseError> {
        Ok(())
    }

    fn read_file(
        &self,
        request: ParseRequest<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
        wanted: &[Relation],
        _: &mut dyn RowWriter,
    ) -> Result<(), ParseError> {
        self.build_file(request, table, symbols, wanted)
    }

    fn read_file_explicit(
        &self,
        request: ParseRequest<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
        wanted: &[Relation],
        out: &mut dyn RowWriter,
    ) -> Result<(), ParseError> {
        self.read_file(request, table, symbols, wanted, out)
    }

    fn walk_file(
        &self,
        _: &mut Elements,
        _: &mut Symbols,
        _: ByteSlice<'_>,
        _: FileId,
        _: &[Relation],
        _: &mut dyn RowWriter,
    ) {
    }

    fn absence_of(&self, _: Relation) -> Option<AbsenceCause> {
        None
    }
}

/// A linter over the language of words, keeping what its runs leave in `cache`, with the engine port's fake in the new
/// engine's place and a compiler of queries that counts the rules it compiles: in `engines` when a row says which run the
/// files, in the ones the environment selects otherwise. The old engine's parse ends each word `short` bytes early.
pub(crate) fn words_linter(
    engines: Option<EngineMode>,
    cache: Cache,
    short: u32,
) -> (Linter, Arc<EngineFake>, Arc<CountingCompiler>) {
    let words = Language::from_parts(
        LanguageName("words".into()),
        LanguageBuild("words-fixture".into()),
        Arc::new(WordsParse { short }),
        Arc::new(ProjectUseCaseImpl),
    )
    .walking(Arc::new(Walking));
    let fake = Arc::new(EngineFake::default());
    let languages = vec![words];
    let build = crate::resources::linter::lint::connections::build(&languages);
    let mut engine = crate::resources::linter::lint::composition::build_engine(
        languages,
        WorkerPool::from(Workers(NonZeroUsize::MIN)),
        build,
        MemoryBudget::Machine,
        cache,
    );
    engine.scan_engine = Arc::<EngineFake>::clone(&fake);
    let compiler = Arc::new(CountingCompiler {
        inner: Arc::clone(&engine.query_language),
        compiled: AtomicUsize::new(0),
    });
    engine.query_language = Arc::<CountingCompiler>::clone(&compiler);
    if let Some(engines) = engines {
        engine.engines = engines;
    }
    (Linter::over(engine), fake, compiler)
}

/// A request over the one listed file `words.walk` of `root`, checked by the rule `words` of `query`.
pub(crate) fn words_request(root: &std::path::Path, query: &str) -> LintRequest {
    let rules = BTreeMap::from([(
        super::rules::rule_id("words"),
        super::rules::ordinary(query),
    )]);
    LintRequest {
        corpus: Corpus::Listed(vec![RelativePath("words.walk".into())]),
        ..super::rules::listed(root, rules)
    }
}
