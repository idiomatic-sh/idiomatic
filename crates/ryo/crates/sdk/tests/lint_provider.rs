//! A faulty provider that is TEST-ONLY: TOML decorated with one named file relation of the shared catalog whose rows are wrong or partial,
//! read through the SDK's public `Linter::lint`. What the executor answers for each fault is what this file pins.

use ryo::languages::contract::ParseEngine;
use ryo::linter::{Cache, Corpus, Languages, LintRequest, Linter, Rule, RuleApplicability, RuleId, SourceLanguage, WorkerPool, Workers};
use ryo::query::RyoQlQuery;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_language::inbound::use_cases::parse::errors::ParseError;
use ryo_language::inbound::use_cases::parse::named::{
    FileProject, FileRelation, FileRelations, FileRows, NumberedFacts, NumberedFile, NumberedReader, NumberedVisit,
};
use ryo_language::domain::facts::types::{FactAnswer, FileWords, FilledColumns, Wanted};
use ryo_language::domain::reasons::types::ToolMessage;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::inbound::use_cases::parse::types::{
    KindNames, ParseConfigurationIdentity, ParseRequest, Recognition, SkippedDirectory,
};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{CellKind, Count, FactColumn, FileId, KindBits, Relation, RowWriter};
use ryo_relations::domain::terms::types::UnknownName;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

/// The relation the faulty provider declares: a relation of the shared catalog (the one Python declares), whose `alias` cell is a node.
static DECLARED: [FileRelation; 1] = [FileRelation {
    name: "import-line-comment",
    columns: &[
        ("statement", CellKind::Node),
        ("comment", CellKind::Node),
        ("first-line", CellKind::Number),
        ("following", CellKind::Number),
        ("prefix", CellKind::Span),
        ("gap", CellKind::Span),
        ("text", CellKind::Symbol),
        ("file", CellKind::File),
    ],
}];

/// What the faulty provider's one row holds: a node of file 0x7fff_ffff, which is no file of the run.
const FOREIGN_NODE: u64 = (0x7fff_ffff_u64 << 32) | 1;

/// How the faulty provider answers its one relation.
#[derive(Clone, Copy)]
enum Fault {
    /// A row whose node is of another file.
    ForeignNode,
    /// A row whose node is of this file but past its last position.
    PastTheEnd,
    /// A row whose span ends after the file does.
    SpanPastTheEnd,
    /// A row whose file cell names a file that is not the one visited.
    OtherFile,
    /// A row that is right, with the relation said to be read in part.
    ReadInPart,
}

struct Faulty {
    numbered: FaultyNumbered,
}

/// The inner language's numbered visit, whose files answer the one named relation as the fault says.
struct FaultyNumbered {
    inner: Arc<dyn ParseEngine>,
    fault: Fault,
}

struct FaultyReader<'a> {
    inner: Box<dyn NumberedReader + 'a>,
    fault: Fault,
}

struct FaultyFacts<'a, 'b> {
    inner: &'a mut (dyn NumberedFacts + 'b),
    fault: Fault,
}

impl NumberedVisit for FaultyNumbered {
    fn fields(&self) -> Option<&ryo_language::domain::fillers::types::FieldNames> {
        self.inner.file_relations().numbered().fields()
    }
    fn projections(&self) -> &'static [Relation] {
        self.inner.file_relations().numbered().projections()
    }
    fn open(&self) -> Box<dyn NumberedReader + '_> {
        let inner = self.inner.file_relations().numbered().open();
        Box::new(FaultyReader { inner, fault: self.fault })
    }
}

impl NumberedReader for FaultyReader<'_> {
    fn visit(
        &mut self,
        request: ParseRequest<'_>,
        catalog: &[Relation],
        named: &[&FileRelation],
        project: Option<&dyn FileProject>,
        symbols: &mut Symbols,
        consume: &mut dyn FnMut(NumberedFile<'_>, &mut Symbols),
    ) -> Result<(), ParseError> {
        let fault = self.fault;
        self.inner.visit(request, catalog, named, project, symbols, &mut |file, symbols| {
            let NumberedFile { tree, words, fact_words, fields, syntax, lexical, notes, facts } = file;
            let mut faulty = FaultyFacts { inner: facts, fault };
            consume(NumberedFile { tree, words, fact_words, fields, syntax, lexical, notes, facts: &mut faulty }, symbols);
        })
    }
}

impl NumberedFacts for FaultyFacts<'_, '_> {
    fn fill_columns(&mut self, relation: Relation, wanted: Wanted, words: &mut FileWords, columns: &mut FilledColumns) -> FactAnswer {
        self.inner.fill_columns(relation, wanted, words, columns)
    }
    fn project_structure(&mut self, relations: &[Relation], symbols: &mut Symbols, out: &mut dyn RowWriter) -> Result<(), ParseError> {
        self.inner.project_structure(relations, symbols, out)
    }
    fn named(&mut self, relations: &[&FileRelation], _: &mut Symbols) -> Vec<Result<FileRows, ParseError>> {
        let fault = self.fault;
        relations
            .iter()
            .map(|_| {
                let columns = (0..8).map(|column| FactColumn(vec![match (column, fault) {
                    (0, Fault::ForeignNode) => FOREIGN_NODE,
                    (0, Fault::PastTheEnd) => 0x00ff_ffff,
                    (4, Fault::SpanPastTheEnd) => 1_000_000,
                    (7, Fault::OtherFile) => 99,
                    (0 | 1, _) => u64::MAX,
                    (4 | 5, _) => u64::from(u32::MAX) << 32,
                    _ => 0,
                }])).collect();
                let partial = matches!(fault, Fault::ReadInPart).then(|| ToolMessage("this file has a construct the reader cannot represent".into()));
                Ok(FileRows { columns, partial })
            })
            .collect()
    }
    fn fills_are_complete(&mut self) -> bool { self.inner.fills_are_complete() }
    fn render(&mut self, request: RenderRequest) -> Result<RenderOutcome, ParseError> { self.inner.render(request) }
}

impl FileRelations for Faulty {
    fn numbered(&self) -> &dyn NumberedVisit {
        &self.numbered
    }
    fn declarations(&self) -> &'static [FileRelation] {
        &DECLARED
    }
    fn read(
        &self,
        relations: &[&FileRelation],
        _: ParseRequest<'_>,
        _: &Elements,
        _: &ryo_language::domain::node_table::types::NodeTable,
        _: &mut Symbols,
        _: Option<&dyn FileProject>,
    ) -> Vec<Result<Vec<FactColumn>, ParseError>> {
        relations.iter().map(|_| Err(ParseError::Library { reason: ToolMessage("the rows come from the numbered visit".into()) })).collect()
    }
}

/// TOML, with one more relation whose rows are wrong.
struct FaultyToml {
    inner: Arc<dyn ParseEngine>,
    faulty: Faulty,
}

impl ParseEngine for FaultyToml {
    fn source_words(&self) -> Option<ryo_language::inbound::use_cases::parse::named::SourceWords> { self.inner.source_words() }
    fn file_relations(&self) -> &dyn FileRelations { &self.faulty }
    fn capabilities(&self) -> Vec<ryo_relations::domain::catalog::types::OptionalCapability> { self.inner.capabilities() }
    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> { self.inner.configuration_identity() }
    fn recognition(&self, request: &ParseRequest<'_>) -> Recognition { self.inner.recognition(request) }
    fn skips(&self) -> &'static [SkippedDirectory] { self.inner.skips() }
    fn kind_count(&self) -> Count { self.inner.kind_count() }
    fn kind_names(&self) -> KindNames { self.inner.kind_names() }
    fn stable_fields(&self) -> ryo_language::domain::fillers::types::FieldNames { self.inner.stable_fields() }
    fn unknown_reasons(&self) -> &'static [UnknownName] { self.inner.unknown_reasons() }
}

fn faulty_toml(fault: Fault) -> ryo::languages::Language {
    let parts: ryo_core::outbound::language::factory::LanguageConnection = ryo_toml::linked().into();
    let numbered = FaultyNumbered { inner: Arc::clone(&parts.parse), fault };
    let faulty = FaultyToml { inner: Arc::clone(&parts.parse), faulty: Faulty { numbered } };
    let linked = ryo::languages::link(ryo::languages::LanguageName(parts.name.0.to_string().into()), ryo::languages::LanguageBuild(parts.build.0.to_string().into()), Arc::new(faulty), parts.project);
    ryo::languages::walking(linked, parts.walk)
}

fn request_for(root: &std::path::Path) -> LintRequest {
    let query = r#"let rows = select * from "import-line-comment";
let named = extend(from=rows, values={"problem": "a row"});
emit named subject "file" key("first-line");"#;
    let rule = Rule {
        observations: Vec::new(),
        query: RyoQlQuery::try_from(query).expect("the query is one"),
        applicability: RuleApplicability::Always,
        parameters: BTreeMap::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    };
    LintRequest {
        source_language: SourceLanguage::Recognized,
        root: root.to_path_buf(),
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        rules: BTreeMap::from([(RuleId::try_from("idiomatic/faulty").expect("id"), rule)]),
        files: Vec::new(),
        hosted: Vec::new(),
        corpus: Corpus::Walked,
        beside: Vec::new(),
    }
}

fn workspace() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a directory");
    std::fs::write(root.path().join("a.toml"), "# one\n").expect("a file");
    root
}

fn lint_over(fault: Fault, root: &std::path::Path) -> Result<ryo::linter::LintReport, ryo::linter::LintError> {
    let linter = Linter::new(Languages(vec![faulty_toml(fault)]), WorkerPool::from(Workers(NonZeroUsize::MIN)), Cache::None);
    linter.lint(vec![request_for(root)]).pop().expect("one answer")
}

/// Old: the core admission of a language's output refused it as `Provider { code }` for each of four faults; the executor does the same.
fn refused_as(fault: Fault, code: ryo::linter::ProviderCode) {
    let root = workspace();
    match lint_over(fault, root.path()) {
        Err(ryo::linter::LintError::Provider { provider, code: found }) => {
            assert_eq!(&*provider.0, "toml");
            assert_eq!(found, code);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_node_of_another_file_is_refused_as_the_provider_s_fault() {
    refused_as(Fault::ForeignNode, ryo::linter::ProviderCode::NodeOfAnotherFile);
}

#[test]
fn a_node_past_the_last_position_of_its_file_is_refused_as_the_provider_s_fault() {
    refused_as(Fault::PastTheEnd, ryo::linter::ProviderCode::NodeOutsideTable);
}

#[test]
fn a_span_past_the_end_of_its_file_is_refused_as_the_provider_s_fault() {
    refused_as(Fault::SpanPastTheEnd, ryo::linter::ProviderCode::SpanOutsideFile);
}

#[test]
fn a_file_cell_that_names_another_file_is_refused_as_the_provider_s_fault() {
    refused_as(Fault::OtherFile, ryo::linter::ProviderCode::FileNotVisited);
}

/// Old: a relation a language says it read only in part leaves an unknown for the file and the lint finishes. The rows that were read stay.
#[test]
fn a_relation_read_in_part_leaves_an_unknown_and_keeps_the_rows_read() {
    let root = workspace();
    let report = lint_over(Fault::ReadInPart, root.path()).expect("answered");
    assert_eq!(report.findings.len(), 1, "{report:?}");
    assert!(report.unread.is_empty() && report.skipped.is_empty(), "{report:?}");
    let unknowns: Vec<_> = report.unknowns.iter().map(|unknown| (format!("{:?}", unknown.path), format!("{:?}", unknown.message))).collect();
    assert_eq!(
        unknowns,
        vec![("RelativePath(\"a.toml\")".to_owned(), "Message(\"this file has a construct the reader cannot represent\")".to_owned())]
    );
}

/// A rule over a relation a language declares is admitted (the compile knows it by its name and columns, as the languages' own relations
/// join the shared catalog), and one over a name nobody declares is refused.
#[test]
fn a_relation_a_language_declares_is_known_and_one_nobody_declares_is_not() {
    let root = workspace();
    assert!(lint_over(Fault::ReadInPart, root.path()).is_ok());
    let mut asked = request_for(root.path());
    for rule in asked.rules.values_mut() {
        rule.query = RyoQlQuery::try_from(r#"let rows = select * from "import-module-nobody";
let named = extend(from=rows, values={"problem": "a row"});
emit named subject "file" key("first-line");"#)
            .expect("the query is one");
    }
    let linter = Linter::new(Languages(vec![faulty_toml(Fault::ReadInPart)]), WorkerPool::from(Workers(NonZeroUsize::MIN)), Cache::None);
    let error = linter.lint(vec![asked]).pop().expect("one answer").expect_err("refused");
    assert!(matches!(error, ryo::linter::LintError::Rule { .. }), "{error:?}");
}

/// A rule over a relation a language declares AND a word for a kind no language has is refused for the kind: the declared relation hides
/// nothing.
#[test]
fn a_declared_relation_does_not_hide_a_kind_no_language_has() {
    let root = workspace();
    let mut asked = request_for(root.path());
    for rule in asked.rules.values_mut() {
        rule.query = RyoQlQuery::try_from(
            r#"let rows = select alias, file, level from "import-line-comment";
let none = select id as alias, file from "syntax-tree" where kind = "no-such-kind";
let kept = antijoin(left=rows, right=none, left-on=["alias","file"], right-on=["alias","file"]);
let named = extend(from=kept, values={"problem": "a row"});
emit named subject "file" key("first-line");"#,
        )
        .expect("the query is one");
    }
    let linter = Linter::new(Languages(vec![faulty_toml(Fault::ReadInPart)]), WorkerPool::from(Workers(NonZeroUsize::MIN)), Cache::None);
    let error = linter.lint(vec![asked]).pop().expect("one answer").expect_err("refused");
    assert!(matches!(error, ryo::linter::LintError::Unrunnable { .. }), "{error:?}");
}
