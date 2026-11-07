//! The corpus the contract runs over: the fixtures of every relation and operator item, and the rows they state.

#![expect(
    clippy::expect_used,
    reason = "a contract run over the corpus fails by panicking: the panic is the failure of the check that calls it"
)]

use crate::domain::corpus_copies::service::copied;
use crate::domain::corpus_copies::types::CopyCount;
use crate::domain::corpus_copies::types::CopyNaming;
use crate::domain::corpus_copies::types::CorpusInputs;
use crate::domain::corpus_copies::types::Extension;
use crate::domain::corpus_copies::types::ItemName;
use crate::domain::corpus_copies::types::ListShape;
use crate::domain::corpus_copies::types::Repeat;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::language::fixtures::fake::READ;
use crate::outbound::language::interface::LanguagePort;
use crate::outbound::language::types::Absence;
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase as _;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::parse::fixtures::expected::Cell;
use ryo_language::inbound::use_cases::parse::fixtures::expected::Fixtures;
use ryo_language::inbound::use_cases::parse::fixtures::expected::Point;
use ryo_relations::domain::relations::service::column_name;
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::service::name;
use ryo_relations::domain::relations::service::provenance_of;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Provenance;
use ryo_values::domain::files::types::CorpusFile;
use ryo_values::domain::files::types::FileBytes;
use ryo_values::domain::query_operators::types::QueryOperator;
use ryo_values::domain::row_texts::types::Cell as Typed;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use super::operators::Finding;
use super::operators::finding;
use super::operators::escaped;
use super::operators::expected;

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

/// The rows each rule is expected to give, sorted, one to a line, by the rule's id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stated(pub(super) BTreeMap<String, String>);


/// Where `point` is in `text`, in bytes.
pub(super) fn offset_of(text: &FileBytes, point: Point) -> usize {
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
pub(super) fn written(cell: &Cell, path: &FilePath, text: &FileBytes) -> String {
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
pub(super) fn compared(cell: Option<&Typed>) -> String {
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
pub(super) fn rendered(findings: &[Finding], path: &FilePath, text: &FileBytes) -> String {
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
pub(super) fn relation_findings(fixtures: &Fixtures, relation: &str) -> Vec<Finding> {
    fixtures
        .relations
        .iter()
        .filter(|each| each.relation == relation)
        .flat_map(|each| each.rows.iter())
        .filter_map(|row| Some(finding(row.first()?.clone(), row.clone())))
        .collect()
}



/// The text of a file of the fixtures, which every row reads whole.
pub(super) fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("the file of the fixtures is read")
}

/// The id of the rule of an operator, and the query its fixtures state with the glob of the files it governs.
pub(super) fn operator_rule(operator: &str, governs: &str) -> (String, (String, String)) {
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
pub(super) fn files(corpus: &Corpus, naming: CopyNaming, copies: u32, repeat: usize) -> Vec<CorpusFile> {
    let (copies, repeat) = (CopyCount(copies), Repeat(repeat));
    let shape = ListShape {
        naming,
        copies,
        repeat,
    };
    copied(&corpus.inputs, &corpus.extension, shape)
}
