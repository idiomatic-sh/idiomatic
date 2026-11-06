// the contract holds the corpus builders (file IO through the language port) and the run-based measuring that the use-case-fixtures template fixes as contract::{allocation, call_count, corpus, fake_corpus}; each of the four is one measurement over the same corpus, so they share its builders.
//! What every implementation of the `lint` use case must pass, and what the whole per-file path must pass: `lint` over the corpus the
//! fixtures state, with the real store and real queries and the language the caller brings. Every relation item the language lists and
//! every operator item the product lists is read from its fixtures, so a new item is covered without anyone editing a test.

#![expect(
    clippy::inline_modules,
    reason = "each operator's expected rows are a file of their own under tests/fixtures, which `--bless` writes; `include!` makes each one a module here, and the module spec declares no file by `#[path]` but a file's own test"
)]
#![expect(
    clippy::expect_used,
    reason = "a contract run over the corpus fails by panicking: the panic is the failure of the check that calls it"
)]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, PoisonError};

use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::parse::fixtures::expected::{Cell, Fixtures, Point};
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;
use ryo_relations::domain::relations::service::{column_name, columns, name, provenance_of};
use ryo_relations::domain::relations::types::{Integer, Provenance};
use ryo_values::domain::files::types::{CorpusFile, FileBytes, Glob};
use ryo_values::domain::plan_parts::types::{LiteralText, ParameterName, QueryText};
use ryo_values::domain::row_texts::types::{Cell as Typed, RowTotal};
use ryo_values::domain::runs::types::{AnswerShape, Session};

use super::super::errors::LintError;
use super::super::interface::LintUseCase;
use super::super::types::{
    LintRequest, ParameterCell, ParameterTable, ParameterValue, Rule, RuleParameter,
};
use super::super::use_case::LintUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run, Matched, selects};
use crate::domain::allocation_verdict::service::{differences, verdict};
use crate::domain::allocation_verdict::types::{
    AllocatedBlocks, AllocationVerdict, InputBytes, JobBlocks, ParserBlocks,
};
use crate::domain::corpus_copies::service::copied;
use crate::domain::corpus_copies::types::{
    CopyCount, CopyDigits, CopyNaming, CorpusInputs, Extension, ItemName, ListShape, Repeat,
};
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::facts::fixtures::fake::FactsFake;
use crate::outbound::language::fixtures::fake::{LanguageFake, READ};
use crate::outbound::language::{interface::LanguagePort, types::Absence};
use crate::outbound::lint_store::interface::LintStorePort;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use crate::outbound::row_sink::fixtures::fake::{BytesRowSinkFake, TallyRowSinkFake};
use crate::outbound::row_sink::interface::RowSinkPort;
use crate::outbound::scheduler::fixtures::fake::{CountingSchedulerFake, SchedulerFake};
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::{facts, matching, query_language, row_sink};
use ryo_execution::inbound::use_cases::execution::types::facts::FactsPort;
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_values::domain::query_operators::types::QueryOperator;

/// What a row of the contract is given: the request to lint.
type Input = LintRequest;
/// What a row of the contract expects when it succeeds: how many files were left unread.
type Value = usize;
/// What a row of the contract expects when it is refused.
type Failure = LintError;

/// What the parameters a fixture query declares are bound to: the layers the functions of the `function` fixture are ranked by in the
/// fixture of `rank`, and the one name the fixture of `rows` holds.
fn fixture_parameters(query: &str) -> Vec<RuleParameter> {
    if query.contains("param names: list<table>;") {
        return vec![RuleParameter {
            name: ParameterName("names".into()),
            value: ParameterValue::Tables(vec![ParameterTable(vec![ParameterCell {
                key: LiteralText("name".into()),
                value: LiteralText("run".into()),
            }])]),
        }];
    }
    if !query.contains("param layers: list<set<string>>;") {
        return Vec::new();
    }
    let layer = |names: &[&str]| -> Vec<LiteralText> {
        names
            .iter()
            .map(|each| LiteralText((*each).into()))
            .collect()
    };
    vec![RuleParameter {
        name: ParameterName("layers".into()),
        value: ParameterValue::TextSets(vec![layer(&["run"]), layer(&["inner", "outer"])]),
    }]
}

/// A rule of this id and query over the files these globs name.
fn rule(id: &str, query: &str, include: Vec<Glob>) -> Rule {
    Rule {
        observations: Vec::new(),
        id: ryo_values::domain::runs::types::RuleId(id.into()),
        query: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(query.into())),
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        parameters: fixture_parameters(query),
        include,
        exclude: Vec::new(),
    }
}

/// A request to lint the repository at the working directory with rules of these ids, whose queries are never run.
fn asking(ids: &[&str]) -> LintRequest {
    LintRequest {
        source_language: None,
        answer: AnswerShape::Findings,
        hosted: Vec::new(),
        session: Session {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
            root: RootPath(".".into()),
        },
        files: Vec::new(),
        rules: ids
            .iter()
            .map(|id| rule(id, "a rule", Vec::new()))
            .collect(),
    }
}

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_rule_governs_no_file_so_none_is_left_unread"),
        word: Word::Succeeds,
        input: || asking(&[]),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_rule_without_an_id_is_refused"),
        word: Word::Refuses,
        input: || asking(&[" "]),
        expect: Expect::Error(LintError::BlankRuleId),
    },
    Case {
        name: CaseName("two_rules_with_one_id_are_refused"),
        word: Word::Refuses,
        input: || asking(&["same", "same"]),
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, LintError::DuplicateRuleId { .. })))
        }),
    },
];

/// Runs the contract's rows against one implementation, in the crate that implements the trait.
pub fn check(subject: &impl LintUseCase) {
    let lints = |input: Input| subject.lint(input).map(|report| report.unread.len());
    run(CASES, Word::Succeeds, lints);
    run(CASES, Word::Refuses, lints);
}

/// How many copies of each input the allocation corpus holds.
const FILES: u32 = 60;
/// How many times the rows of every file the larger call-count corpus has.
const TIMES: usize = 10;

/// What the fixtures state: the input of each relation item, and the rule and the rows of each item.
pub struct Corpus {
    /// The language that reads the inputs.
    pub language: Arc<dyn LanguagePort>,
    /// The input of each relation item, by the item's name.
    pub inputs: CorpusInputs,
    /// The query and the glob of the files it governs, by the rule's id, `relation:<name>` or `operator:<name>`: a relation's rule
    /// governs its own input, an operator's the input of the `function` relation, which the operators are run over.
    pub rules: BTreeMap<String, (String, String)>,
    /// The rows each rule gives over its files, one copy of each, by the rule's id.
    pub expected: Stated,
    /// How the files of the language end, without the dot.
    pub extension: Extension,
}

impl std::fmt::Debug for Corpus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Corpus")
            .field("inputs", &self.inputs)
            .field("rules", &self.rules)
            .field("expected", &self.expected)
            .field("extension", &self.extension)
            .finish_non_exhaustive()
    }
}

/// One finding the rule of an operator gives: what it is about, and the cells of its key.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Finding {
    /// The cell the finding is about, written first in its row.
    subject: Cell,
    /// The cells that tell this finding from another about the same subject.
    key: Vec<Cell>,
}

/// The rows each rule is expected to give, sorted, one to a line, by the rule's id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stated(BTreeMap<String, String>);

/// The rows the rules gave, one line each, by the rule's id.
type Given = BTreeMap<String, Vec<String>>;

/// A finding about `subject` with `key`.
fn finding(subject: impl Into<Cell>, key: Vec<impl Into<Cell>>) -> Finding {
    Finding {
        subject: subject.into(),
        key: key.into_iter().map(Into::into).collect(),
    }
}

/// A cell of a key as the rows of a rule write it, one row to a line: a tab and a line break are `\t`, `\n` and `\r`.
fn escaped(text: &str) -> String {
    text.replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Where `point` is in `text`, in bytes.
fn offset_of(text: &FileBytes, point: Point) -> usize {
    let mut line_start = 0;
    for _ in 1..point.line {
        line_start += text
            .0
            .get(line_start..)
            .and_then(|rest| rest.iter().position(|byte| *byte == b'\n'))
            .map_or(0, |at| at + 1);
    }
    line_start + usize::try_from(point.column.saturating_sub(1)).unwrap_or(0)
}

/// A cell of a finding as the rows of a rule write it: a node as the path of its file and its place, a span as the bytes it covers
/// in `text`.
fn written(cell: &Cell, path: &FilePath, text: &FileBytes) -> String {
    match cell {
        Cell::Node(place) => format!("{}#{place}", path.0),
        Cell::Name(name) => escaped(name),
        Cell::Unknown(reason) => format!("?{reason}"),
        Cell::File(file) => file.clone(),
        Cell::Unlocated => "unlocated".to_owned(),
        Cell::Span(from, to) => format!("{}..{}", offset_of(text, *from), offset_of(text, *to)),
        Cell::Number(Integer(number)) => number.to_string(),
        Cell::Flag(flag) => flag.to_string(),
    }
}

/// A cell the run gave, as the rows of the fixtures write it: a file and a text as they are, a number in decimal, a span as its two
/// ends, a cell with no value as nothing.
fn compared(cell: Option<&Typed>) -> String {
    match cell {
        Some(Typed::File(text) | Typed::Text(text)) => text.to_string(),
        Some(Typed::Number(number)) => number.0.to_string(),
        Some(Typed::Span(span))
            if ryo_relations::domain::relations::service::location(*span)
                == ryo_relations::domain::relations::types::Locatedness::Absent =>
        {
            "unlocated".to_owned()
        }
        Some(Typed::Span(span)) => format!("{}..{}", span.start.0, span.end.0),
        None => String::new(),
    }
}

/// The rows the findings are written as, sorted, one to a line.
fn rendered(findings: &[Finding], path: &FilePath, text: &FileBytes) -> String {
    let mut lines: Vec<String> = findings
        .iter()
        .map(|each| {
            let key: Vec<String> = each
                .key
                .iter()
                .map(|cell| written(cell, path, text))
                .collect();
            format!("{}\t{}", written(&each.subject, path, text), key.join(","))
        })
        .collect();
    lines.sort();
    lines.join("\n")
}

/// The findings the fixtures state for the relation of this folder: every row of it, about its first cell.
fn relation_findings(fixtures: &Fixtures, relation: &str) -> Vec<Finding> {
    fixtures
        .relations
        .iter()
        .filter(|each| each.relation == relation)
        .flat_map(|each| each.rows.iter())
        .filter_map(|row| Some(finding(row.first()?.clone(), row.clone())))
        .collect()
}

/// Files the row a rule gave, as a line of `path`, a tab and the escaped `key`, under the rule.
fn record(given: &mut Given, rule: &str, path: &str, key: &str) {
    given
        .entry(rule.to_owned())
        .or_default()
        .push(format!("{path}\t{}", escaped(key)));
}

/// The rules whose rows, sorted, are not the rows stated for them, each with the rows it gave; empty when every rule agrees.
fn disagreements(given: &Given, stated: &Stated) -> String {
    let mut wrong = Vec::new();
    for (id, expected) in &stated.0 {
        let mut rows: Vec<&str> = given
            .get(id)
            .map(|lines| lines.iter().map(String::as_str).collect())
            .unwrap_or_default();
        rows.sort_unstable();
        let rows = rows.join("\n");
        if rows != expected.trim_end() {
            wrong.push(format!("== {id}\n{rows}"));
        }
    }
    wrong.join("\n")
}

/// The text of a file of the fixtures, which every row reads whole.
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("the file of the fixtures is read")
}

/// The id of the rule of an operator, and the query its fixtures state with the glob of the files it governs.
fn operator_rule(operator: &str, governs: &str) -> (String, (String, String)) {
    let folder = operator.replace('-', "_");
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/operators");
    let query = read(&path.join(folder).join("input.ryoql"));
    (format!("operator:{operator}"), (query, governs.to_owned()))
}

/// The corpus of the core's own: the fake language, which reads a file as a few functions, and the rule of every operator over
/// them. It has no expected rows, for the allocation row measures what the path allocates, not what it finds.
///
/// # Panics
///
/// When two operators have one name, and so one rule.
#[must_use]
pub fn fake_corpus() -> Corpus {
    let extension = Extension(READ.trim_start_matches('.').to_owned());
    let language = LanguageFake {
        extra_rows: 8,
        constant_name: true,
        ..Default::default()
    };
    let mut found = Corpus {
        language: Arc::new(language),
        inputs: CorpusInputs::default(),
        rules: BTreeMap::new(),
        expected: Stated::default(),
        extension: extension.clone(),
    };
    let function = FileBytes(b"fn main() {}".to_vec());
    drop(
        found
            .inputs
            .0
            .insert(ItemName("function".to_owned()), function),
    );
    for &operator in MetadataUseCaseImpl.operators() {
        // This fake parser has no semantic-rendering session. The language
        // capability is exercised by the Python provider's actual case run.
        if matches!(operator, QueryOperator::RenderSyntax) {
            continue;
        }
        let governs = format!("function/*.{}", extension.0);
        let (id, rule) = operator_rule(&MetadataUseCaseImpl.describe(operator).name.0, &governs);
        assert!(
            found.rules.insert(id, rule).is_none(),
            "every operator has one rule"
        );
    }
    found
}

/// Reads the fixtures of every item: a relation's from the language's `tests/fixtures`, an operator's from the core's own; the rows a
/// relation's rule gives are the rows the language expects of its fixture, and an operator's the findings stated for it. A
/// relation's rule gives every column of its rows.
///
/// # Panics
///
/// When an item has no fixtures.
#[must_use]
pub fn corpus(
    language: Arc<dyn LanguagePort>,
    language_fixtures: &Path,
    fixtures: &Fixtures,
) -> Corpus {
    let mut found = Corpus {
        language,
        inputs: CorpusInputs::default(),
        rules: BTreeMap::new(),
        expected: Stated::default(),
        extension: Extension("rs".to_owned()),
    };
    let relations = found
        .language
        .relations()
        .expect("the relations are listed");
    for relation in relations.0 {
        // A relation no linked language reads has no fixture, and the manifests' relations are the project's.
        if provenance_of(relation) != Provenance::Trees
            || (found.language.supports(relation).0.iter())
                .all(|answer| answer.absence != Absence::Reads)
        {
            continue;
        }
        let item = name(relation).0;
        let folder = item.replace('-', "_");
        let path = language_fixtures.join("relations").join(&folder);
        let input = FileBytes(read(&path.join("input.rs")).into_bytes());
        let id = format!("relation:{item}");
        let stated = relation_findings(fixtures, &folder);
        let shown = FilePath(format!("{folder}/input.rs").into());
        let rows = rendered(&stated, &shown, &input);
        assert!(
            found.expected.0.insert(id.clone(), rows).is_none(),
            "every relation states its rows once"
        );
        assert!(
            found
                .inputs
                .0
                .insert(ItemName(item.to_owned()), input)
                .is_none(),
            "every relation has one input"
        );
        let names: Vec<&str> = columns(relation)
            .into_iter()
            .map(|column| column_name(column).0)
            .collect();
        let query = format!(
            "let a = select * from \"{item}\"; emit a subject {} key({});",
            names.first().copied().unwrap_or_default(),
            names.join(", ")
        );
        assert!(
            found
                .rules
                .insert(id, (query, format!("{folder}/*.rs")))
                .is_none(),
            "every relation has one rule"
        );
    }
    let function = ItemName("function".to_owned());
    let text = found.inputs.0.get(&function).cloned().unwrap_or_default();
    for &operator in MetadataUseCaseImpl.operators() {
        // The Rust fixture parser deliberately has no semantic-rendering
        // session; its other generic operators remain in this corpus.
        if matches!(operator, QueryOperator::RenderSyntax) {
            continue;
        }
        let (id, rule) = operator_rule(
            &MetadataUseCaseImpl.describe(operator).name.0,
            "function/*.rs",
        );
        let shown = FilePath("function/input.rs".into());
        let stated = expected(operator);
        assert!(!stated.is_empty(), "every operator states its findings");
        let rows = rendered(&stated, &shown, &text);
        assert!(
            found.expected.0.insert(id.clone(), rows).is_none(),
            "every operator states its findings once"
        );
        assert!(
            found.rules.insert(id, rule).is_none(),
            "every operator has one rule"
        );
    }
    found
}

/// The files of every input of the corpus, once for each of `copies` copies, named as `naming` says, with the bytes repeated `repeat` times.
fn files(corpus: &Corpus, naming: CopyNaming, copies: u32, repeat: usize) -> Vec<CorpusFile> {
    let (copies, repeat) = (CopyCount(copies), Repeat(repeat));
    let shape = ListShape {
        naming,
        copies,
        repeat,
    };
    copied(&corpus.inputs, &corpus.extension, shape)
}

/// The compiler of queries the use cases are given.
fn compile() -> CompileUseCaseImpl {
    CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    )
}

/// The typechecker of queries the use cases are given.
fn typecheck() -> TypecheckUseCaseImpl {
    TypecheckUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    )
}

/// Lints `files` with every rule of the corpus, the scheduler and the facts port given, into `rows`.
fn lint(
    corpus: &Corpus,
    scheduler: Arc<dyn SchedulerPort>,
    facts: Arc<dyn FactsPort>,
    files: Vec<CorpusFile>,
    rows: Arc<dyn RowSinkPort>,
) {
    lint_keeping(
        corpus,
        scheduler,
        facts,
        files,
        rows,
        Arc::new(crate::outbound::lint_store::factory::memory()),
    );
}

/// The same lint, keeping its findings in `store`.
fn lint_keeping(
    corpus: &Corpus,
    scheduler: Arc<dyn SchedulerPort>,
    facts: Arc<dyn FactsPort>,
    files: Vec<CorpusFile>,
    rows: Arc<dyn RowSinkPort>,
    store: Arc<dyn LintStorePort>,
) {
    let use_case = use_case_over(corpus, scheduler, facts, files, rows, store);
    let rules = corpus.rules.iter();
    let rules =
        rules.map(|(id, (query, governs))| rule(id, query, vec![Glob(governs.as_str().into())]));
    let request = LintRequest {
        source_language: None,
        rules: rules.collect(),
        ..asking(&[])
    };
    let report = use_case.lint(request).expect("the corpus lints");
    assert_eq!(report.unread.len(), 0, "every file of the corpus is read");
}

/// The `lint` use case over `files` and the language of the corpus, with the scheduler and the facts port given, writing into
/// `rows` and keeping its findings in `store`.
fn use_case_over(
    corpus: &Corpus,
    scheduler: Arc<dyn SchedulerPort>,
    facts: Arc<dyn FactsPort>,
    files: Vec<CorpusFile>,
    rows: Arc<dyn RowSinkPort>,
    store: Arc<dyn LintStorePort>,
) -> LintUseCaseImpl {
    {
        let analysis_corpus: Arc<dyn crate::outbound::corpus::interface::CorpusPort> =
            Arc::new(CorpusFake {
                files,
                fails: false,
                reads: Arc::default(),
                outside: Vec::new(),
            });
        let analysis_language: Arc<dyn LanguagePort> = Arc::clone(&corpus.language);
        let analysis_memory: Arc<dyn crate::outbound::memory::interface::MemoryPort> =
            Arc::new(MemoryFake::default());
        let analysis_digest: Arc<
            dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort,
        > = Arc::new(crate::outbound::digest::fixtures::fake::DigestFake);
        LintUseCaseImpl::new(
            crate::application::file_analysis::service::FileAnalysis::new(
                Arc::clone(&analysis_corpus),
                Arc::clone(&analysis_language),
                Arc::clone(&analysis_digest),
                Arc::clone(&analysis_memory),
            ),
            crate::application::cross_file::service::CrossFile::new(Arc::clone(&analysis_corpus), std::sync::Arc::new(crate::outbound::publication::factory::execution(std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new())))),
            Arc::clone(&analysis_corpus),
            facts,
            Arc::clone(&analysis_language),
            Arc::new(matching::factory::globset()),
            Arc::new(query_language::factory::use_case(
                Arc::new(compile()),
                Arc::new(typecheck()),
            )),
            Arc::clone(&analysis_memory),
            Arc::new(crate::outbound::operators::factory::use_case(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
            )),
            std::sync::Arc::new(crate::outbound::derived::factory::use_case(
                ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
            )),
            scheduler,
            crate::outbound::scheduler::types::WorkerCount(1),
            rows,
            Arc::clone(&analysis_digest),
            store,
        )
    }
}

/// The rows each rule gives over one copy of every input, by the rule's id.
fn given(corpus: &Corpus, scheduler: Arc<dyn SchedulerPort>) -> Given {
    let collected = row_sink::types::Collector::default();
    let kept: Arc<dyn RowSinkPort> = Arc::new(row_sink::factory::collect(collected.clone()));
    let files = files(corpus, CopyNaming::Input, 1, 1);
    lint(
        corpus,
        scheduler,
        Arc::new(facts::factory::memory()),
        files,
        Arc::clone(&kept),
    );
    let mut given = Given::new();
    for row in
        &std::mem::take(&mut *collected.0.lock().unwrap_or_else(PoisonError::into_inner)).rows
    {
        // A row is the rule, the subject, how many key values and each, then its headed columns, line and column.
        if let [
            Some(Typed::Text(rule)),
            path,
            Some(Typed::Number(count)),
            rest @ ..,
        ] = &*row.0
        {
            let key: Vec<String> = rest
                .iter()
                .take(usize::try_from(count.0).unwrap_or(0))
                .map(|cell| compared(cell.as_ref()))
                .collect();
            record(&mut given, rule, &compared(path.as_ref()), &key.join(","));
        }
    }
    given
}

/// Checks that each rule gave the rows its fixture states.
fn agrees(given: &Given, corpus: &Corpus) {
    let wrong = disagreements(given, &corpus.expected);
    assert!(
        wrong.is_empty(),
        "these rules do not give the rows their fixtures state, which are:\n{wrong}"
    );
}

/// The allocation row of the whole per-file path: a file allocates no more than the parser library
/// does on its own, after warm-up. Core, store and language add nothing, but for the findings the
/// files leave, a vector that doubles now and then, so a few files, logarithmic in their number,
/// allocate one block more. `library` does only what the parser library does for a file.
///
/// # Panics
///
/// When a rule does not give its fixture's rows, or a file allocates more than allowed.
pub fn allocation(corpus: &Corpus, library: &dyn Fn(&[u8])) {
    let _profiler = dhat::Profiler::builder().testing().build();
    let scheduler = Arc::new(CountingSchedulerFake::default());
    let once = given(corpus, Arc::<CountingSchedulerFake>::clone(&scheduler));
    agrees(&once, corpus);

    // The copies: the first lint grows what a run keeps, the way a program's first files do; the second is the one counted.
    let copies = || {
        files(
            corpus,
            CopyNaming::Copy {
                digits: CopyDigits(3),
            },
            FILES,
            1,
        )
    };
    let counted = Arc::new(TallyRowSinkFake::default());
    for sink in [Arc::new(TallyRowSinkFake::default()), Arc::clone(&counted)] {
        let facts = Arc::new(facts::factory::memory());
        lint(
            corpus,
            Arc::<CountingSchedulerFake>::clone(&scheduler),
            facts,
            copies(),
            sink,
        );
    }
    let counted = counted
        .counts
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    for (id, rows) in &once {
        let every_copy = rows.len() * usize::try_from(FILES).unwrap_or(usize::MAX);
        let gave = counted.get(id).copied().unwrap_or(0);
        assert_eq!(gave, every_copy, "{id} gives its rows for every copy");
    }
    let jobs = scheduler
        .blocks
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();

    // The parser library alone, for the file each job read: the files are taken in the order of their paths.
    let mut listed = copies();
    listed.sort_by(|left, right| left.path.cmp(&right.path));
    let mut parser = ParserBlocks::default();
    for file in &listed {
        let content = InputBytes(file.bytes.0.clone());
        let _: &mut AllocatedBlocks = parser.0.entry(content).or_insert_with(|| {
            library(&file.bytes.0);
            let before = dhat::HeapStats::get().total_blocks;
            library(&file.bytes.0);
            AllocatedBlocks(dhat::HeapStats::get().total_blocks - before)
        });
    }
    let gaps = differences(&JobBlocks(jobs), &listed, &parser);
    let beyond = match verdict(&gaps, CopyCount(FILES)) {
        AllocationVerdict::Within => None,
        AllocationVerdict::BeyondOneBlock => Some(format!(
            "a file allocated more than one block beyond the parser library: {:?}",
            gaps.0
        )),
        AllocationVerdict::TooManyGrew { grew, judged } => Some(format!(
            "{} of {} files allocated beyond the parser library, more than the findings vector doubles: {:?}",
            grew.0, judged.0, gaps.0
        )),
    };
    assert_eq!(
        beyond, None,
        "the per-file path allocates as the parser library does"
    );
    streaming(corpus);
    output_path(corpus, library);
}

/// The call-count row of the whole per-file path: a port is called for a file, never for a row.
/// The corpus and the same corpus with ten times the rows in every file call the facts port as often.
/// Its rules give the rows their fixtures state first, so a fixture nothing gives fails here.
///
/// # Panics
///
/// When a rule does not give its fixture's rows, the larger files do not have more rows, or the
/// port was called more often for them.
pub fn call_count(corpus: &Corpus) {
    // The rows first: each rule of the corpus gives what its fixture states, the operators' and the relations'.
    agrees(&given(corpus, Arc::new(SchedulerFake::default())), corpus);
    let counted = |times: usize| -> (usize, usize) {
        let facts = Arc::new(FactsFake::default());
        let found = Arc::new(TallyRowSinkFake::default());
        let files = files(corpus, CopyNaming::Input, 1, times);
        lint(
            corpus,
            Arc::new(SchedulerFake::default()),
            Arc::<FactsFake>::clone(&facts),
            files,
            Arc::<TallyRowSinkFake>::clone(&found),
        );
        let tallies = found.counts.lock().unwrap_or_else(PoisonError::into_inner);
        (tallies.values().sum(), facts.calls())
    };
    let ((few, few_calls), (many, many_calls)) = (counted(1), counted(TIMES));
    assert!(many > few, "the larger files do have more rows");
    assert_eq!(
        many_calls, few_calls,
        "the facts port was called more often for the larger files"
    );
}

/// What a lint and a query wrote, and what the lint's store holds of its findings once the run is over.
struct Written {
    /// The bytes written.
    bytes: usize,
    /// The bytes the store holds.
    kept: usize,
}

/// Reads `copies` copies of every input, lints them and runs a query over them, and answers how
/// many bytes were written and how many the lint's store holds.
fn write_all(corpus: &Corpus, copies: u32) -> Written {
    let linted = Arc::new(BytesRowSinkFake::default());
    let files = files(
        corpus,
        CopyNaming::Copy {
            digits: CopyDigits(5),
        },
        copies,
        1,
    );
    let facts = Arc::new(facts::factory::memory());
    let store: Arc<dyn LintStorePort> = Arc::new(crate::outbound::lint_store::factory::memory());
    lint_keeping(
        corpus,
        Arc::new(SchedulerFake::default()),
        facts,
        files,
        Arc::<BytesRowSinkFake>::clone(&linted),
        Arc::clone(&store),
    );
    // What the store holds is what letting it go gives back.
    let holding = dhat::HeapStats::get().curr_bytes;
    drop(store);
    let kept = holding.saturating_sub(dhat::HeapStats::get().curr_bytes);
    let written = Arc::new(BytesRowSinkFake::default());
    run_query(
        corpus,
        copies,
        1,
        u64::MAX,
        Arc::<BytesRowSinkFake>::clone(&written),
    );
    Written {
        bytes: linted.bytes.load(Ordering::Relaxed) + written.bytes.load(Ordering::Relaxed),
        kept,
    }
}

/// Runs the query over `copies` copies of every input, each as long as `times` of it, writing at most `limit` rows into `written`.
fn run_query(
    corpus: &Corpus,
    copies: u32,
    times: usize,
    limit: u64,
    written: Arc<BytesRowSinkFake>,
) {
    let files = files(
        corpus,
        CopyNaming::Copy {
            digits: CopyDigits(5),
        },
        copies,
        times,
    );
    let use_case = use_case_over(
        corpus,
        Arc::new(SchedulerFake::default()),
        Arc::new(facts::factory::memory()),
        files,
        written,
        Arc::new(crate::outbound::lint_store::factory::memory()),
    );
    let text = "let a = select * from \"syntax-tree\"; emit a subject file key(span);";
    let request = LintRequest {
        source_language: None,
        rules: vec![rule("query", text, Vec::new())],
        answer: AnswerShape::Rows(RowTotal(limit)),
        ..asking(&[])
    };
    let report = use_case.lint(request).expect("the corpus runs");
    assert!(report.unread.is_empty(), "every file of the corpus is read");
}

/// The streaming row: what `run` and `lint` write does not wait in memory. Ten times the files
/// write ten times the bytes, and the most memory the process holds grows, beyond the findings the
/// lint keeps in its store for the next run, by less than a fifth of the bytes written, whatever the
/// rows are: the files themselves are what grows.
///
/// # Panics
///
/// When the larger run holds on to what it wrote.
pub fn streaming(corpus: &Corpus) {
    let warm = write_all(corpus, FILES);
    let before = dhat::HeapStats::get().max_bytes;
    let Written {
        bytes: written,
        kept,
    } = write_all(corpus, FILES * 10);
    assert!(written > warm.bytes, "ten times the files write more bytes");
    let grown = dhat::HeapStats::get()
        .max_bytes
        .saturating_sub(before)
        .saturating_sub(kept);
    assert!(
        written > 0 && grown * 5 < written,
        "the larger run grew the peak by {grown} bytes beyond the {kept} its store keeps while writing {written}: its output is held, not streamed"
    );
}

/// The output-path row: once the workers have grown their buffers, writing the rows of more files with ten times the rows allocates
/// nothing, but for the buffers that double now and then. Rows are formatted into the buffers a worker reuses, never into a string or
/// a vector of their own.
///
/// # Panics
///
/// When more files allocate more blocks than the doubling of the buffers explains.
pub(crate) fn output_path(corpus: &Corpus, library: &dyn Fn(&[u8])) {
    let blocks = || dhat::HeapStats::get().total_blocks;
    let beyond = |copies: u32, limit: u64| -> i64 {
        let before = blocks();
        run_query(
            corpus,
            copies,
            TIMES,
            limit,
            Arc::new(BytesRowSinkFake::default()),
        );
        let run = i64::try_from(blocks() - before).unwrap_or(i64::MAX);
        let parsing = blocks();
        for _ in 0..copies {
            for bytes in corpus.inputs.0.values() {
                library(&bytes.0.repeat(TIMES));
            }
        }
        run - i64::try_from(blocks() - parsing).unwrap_or(i64::MAX)
    };
    // Everything a file costs beyond the parser is the evaluation's, which the rows do not change:
    // what writing the rows costs is the difference between a run that writes them and one that writes none.
    let written = beyond(FILES * 4, u64::MAX) - beyond(FILES, u64::MAX);
    let counted = beyond(FILES * 4, 0) - beyond(FILES, 0);
    let extra = written - counted;
    assert!(
        extra <= 16 * i64::from(FILES.ilog2()),
        "{} more files with ten times the rows allocated {extra} blocks to write their rows: the output path allocates for rows",
        FILES * 3
    );
}

/// The findings the rule of `operator` gives over the `function` fixture, in any order, as the operator's
/// `tests/fixtures/operators/<item>/expected.rs` states them.
fn expected(operator: QueryOperator) -> Vec<Finding> {
    match operator {
        QueryOperator::Replace => replace::expected(),
        QueryOperator::SpanBetween => span_between::expected(),
        QueryOperator::SourceMatches => source_matches::expected(),
        QueryOperator::RenderSyntax => {
            unreachable!("a language without semantic rendering does not register this fixture")
        }
        QueryOperator::FitsRendered => fits_rendered::expected(),
        QueryOperator::SpanBounds => span_bounds::expected(),
        QueryOperator::OrderedMatch => ordered_match::expected(),
        QueryOperator::OrderedDfs => ordered_dfs::expected(),
        QueryOperator::Antijoin => antijoin::expected(),
        QueryOperator::Axis => axis::expected(),
        QueryOperator::Capture => capture::expected(),
        QueryOperator::Closure => closure::expected(),
        QueryOperator::Cycles => cycles::expected(),
        QueryOperator::Difference => difference::expected(),
        QueryOperator::Extend => extend::expected(),
        QueryOperator::GroupCount => group_count::expected(),
        QueryOperator::GroupDigest => group_digest::expected(),
        QueryOperator::Intersection => intersection::expected(),
        QueryOperator::Join => join::expected(),
        QueryOperator::Project => project::expected(),
        QueryOperator::Rank => rank::expected(),
        QueryOperator::Rows => rows_operator::expected(),
        QueryOperator::Select => select::expected(),
        QueryOperator::Union => union::expected(),
        QueryOperator::Words => words::expected(),
    }
}

/// The findings `tests/fixtures/operators/antijoin/expected.rs` states for the `antijoin` operator.
mod antijoin {
    include!("../../../../../tests/fixtures/operators/antijoin/expected.rs");
}

/// The findings `tests/fixtures/operators/rows/expected.rs` states for the `rows` operator.
mod rows_operator {
    include!("../../../../../tests/fixtures/operators/rows/expected.rs");
}

/// The findings `tests/fixtures/operators/axis/expected.rs` states for the `axis` operator.
mod axis {
    include!("../../../../../tests/fixtures/operators/axis/expected.rs");
}

/// The findings `tests/fixtures/operators/capture/expected.rs` states for the `capture` operator.
mod capture {
    include!("../../../../../tests/fixtures/operators/capture/expected.rs");
}

/// The findings `tests/fixtures/operators/closure/expected.rs` states for the `closure` operator.
mod closure {
    include!("../../../../../tests/fixtures/operators/closure/expected.rs");
}

/// The findings `tests/fixtures/operators/cycles/expected.rs` states for the `cycles` operator.
mod cycles {
    include!("../../../../../tests/fixtures/operators/cycles/expected.rs");
}

/// The findings `tests/fixtures/operators/difference/expected.rs` states for the `difference` operator.
mod difference {
    include!("../../../../../tests/fixtures/operators/difference/expected.rs");
}

/// The findings `tests/fixtures/operators/extend/expected.rs` states for the `extend` operator.
mod extend {
    include!("../../../../../tests/fixtures/operators/extend/expected.rs");
}

/// The findings `tests/fixtures/operators/group_count/expected.rs` states for the `group-count` operator.
mod group_count {
    include!("../../../../../tests/fixtures/operators/group_count/expected.rs");
}

/// The findings `tests/fixtures/operators/group_digest/expected.rs` states for the `group-digest` operator.
mod group_digest {
    include!("../../../../../tests/fixtures/operators/group_digest/expected.rs");
}

/// The findings `tests/fixtures/operators/intersection/expected.rs` states for the `intersection` operator.
mod intersection {
    include!("../../../../../tests/fixtures/operators/intersection/expected.rs");
}

/// The findings `tests/fixtures/operators/join/expected.rs` states for the `join` operator.
mod join {
    include!("../../../../../tests/fixtures/operators/join/expected.rs");
}

/// The findings `tests/fixtures/operators/project/expected.rs` states for the `project` operator.
mod project {
    include!("../../../../../tests/fixtures/operators/project/expected.rs");
}

/// The findings `tests/fixtures/operators/rank/expected.rs` states for the `rank` operator.
mod rank {
    include!("../../../../../tests/fixtures/operators/rank/expected.rs");
}

/// The findings `tests/fixtures/operators/select/expected.rs` states for the `select` operator.
mod select {
    include!("../../../../../tests/fixtures/operators/select/expected.rs");
}

/// The findings `tests/fixtures/operators/union/expected.rs` states for the `union` operator.
mod union {
    include!("../../../../../tests/fixtures/operators/union/expected.rs");
}

/// The findings `tests/fixtures/operators/words/expected.rs` states for the `words` operator.
mod words {
    include!("../../../../../tests/fixtures/operators/words/expected.rs");
}

/// Literal replacement fixture, independent expected transformed names.
mod replace {
    include!("../../../../../tests/fixtures/operators/replace/expected.rs");
}

/// Independent byte span projection fixture.
mod span_between {
    include!("../../../../../tests/fixtures/operators/span_between/expected.rs");
}
/// Independent physical occurrence spans, including the duplicate nested occurrence.
mod source_matches {
    include!("../../../../../tests/fixtures/operators/source_matches/expected.rs");
}
mod fits_rendered {
    include!("../../../../../tests/fixtures/operators/fits_rendered/expected.rs");
}
mod span_bounds {
    include!("../../../../../tests/fixtures/operators/span_bounds/expected.rs");
}
/// Ordered request/reply fixture, preserving explicit output projection.
mod ordered_match {
    include!("../../../../../tests/fixtures/operators/ordered_match/expected.rs");
}

/// The ordered DFS fixture's complete expected visits.
mod ordered_dfs {
    include!("../../../../../tests/fixtures/operators/ordered_dfs/expected.rs");
}
