// ignore: idiomatic.arch/file-length - the one entry_test.rs an entry has holds a row for each promise the entry makes to its caller, each with the repository it runs over; a second test file for one entry is a role no spec places
//! What `linter lint` promises its caller, one row of `CASES` for each case it has.

use std::collections::BTreeMap;
use std::num::{NonZeroU64, NonZeroUsize};

use super::entry::lint;
use super::errors::LintError;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::capacity::types::MemoryBudget;
use crate::domain::cell::types::{Cell, HostNumber, HostText};
use crate::domain::file::types::RelativePath;
use crate::domain::host::types::{HostColumn, HostColumnName, HostKind, HostRelationName};
use crate::domain::language::types::Languages;
use crate::resources::linter::lint::types::Cache;
use crate::resources::linter::lint::types::CaseSource;
use crate::resources::linter::lint::types::Corpus;
use crate::resources::linter::lint::types::HostRelation;
use crate::resources::linter::lint::types::HostRow;
use crate::resources::linter::lint::types::LintRequest;
use crate::resources::linter::lint::types::Rule;
use crate::resources::linter::lint::types::RuleId;
use crate::resources::linter::lint::types::SourceText;
use crate::resources::linter::lint::types::Workers;

/// Where the repository is.
#[derive(Clone, Copy, Debug)]
enum Root {
    /// A directory holding one file.
    Directory,
    /// A directory that is not there.
    Missing,
    /// A path that is not text. A Unix path from raw bytes and a Windows path from a lone surrogate can be other than
    /// text, so the row and this variant exist there.
    #[cfg(any(unix, windows))]
    NotText,
}

/// Which linter the row's calls go to.
#[derive(Clone, Copy, Debug)]
enum Linting {
    /// The one call, to a linter of its own.
    Fresh,
    /// Two different calls to one linter, one after the other, interleaved and in parallel, each checked against what the same
    /// call gives on a fresh linter: the value is the first call's.
    Shared,
    /// Same-binary optional capability changes invalidate retained observations.
    Capabilities,
}

/// What the caller states: the rule's text, whether the run keeps what it leaves in a directory of the caller's, where the repository
/// is, the most memory the run may use in bytes (none: the machine's), whether the file it holds is said to be placed, and whether the
/// files are given as text, and which linter the calls go to.
#[derive(Clone, Copy, Debug)]
struct Given {
    query: &'static str,
    kept: bool,
    root: Root,
    bytes: Option<u64>,
    hosted: Hosting,
    corpus: Reads,
    linter: Linting,
}

/// Which files the run is told to read.
#[derive(Clone, Copy, Debug)]
enum Reads {
    /// The disk, walked.
    Walked,
    /// The disk, as listed: the one file there.
    Listed,
    /// The disk, as listed: nothing.
    ListedNothing,
    /// The disk, walked, and a file given as text beside it.
    Beside,
    /// Only a file given as text.
    Given,
}

/// What the host declares with the request.
#[derive(Clone, Copy, Debug)]
enum Hosting {
    /// Nothing.
    None,
    /// A relation of the files it placed, its rows right.
    Placed,
    /// A row with a cell fewer than the relation has columns.
    ShortRow,
    /// A cell that is not of the kind of its column.
    WrongKind,
    /// Two relations with one name.
    Twice,
}

/// What a row gives the function: the rule's text, where the repository is and what else the caller states.
type Input = Given;
/// How many findings, how many files nothing read, whether the repository holds nothing but what it held, and how many columns the
/// first finding carries.
type Value = (usize, usize, bool, usize);
/// Why the function ends in error.
type Failure = Refusal;

/// The refusals the rows tell apart.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    /// Preserve the engine execution reason for an unexpected runtime refusal.
    Execution(String),
    /// Invalid declared observation bindings.
    ObservationBindings,
    /// Required observation publication failed during execution.
    ObservationExecution,
    /// The explicitly requested source provider is not linked.
    SourceLanguage,
    /// A rule whose text is wrong.
    Rule,
    /// A rule that cannot be run.
    Unrunnable,
    /// A parameter nothing is bound to.
    MissingParameter,
    /// A repository that does not fit.
    TooLarge,
    /// A path that is not text.
    PathNotText,
    /// A part of the system failed.
    Io,
    /// A language wrote what its file cannot hold.
    Provider,
    /// The host declared its relations wrongly.
    Host,
    /// The run held more than the budget.
    MemoryBudgetExceeded,
}

const FUNCTIONS: &str =
    "let functions = select * from \"function\"; emit functions subject \"file\" key(\"span\");";
const HOSTED: &str =
    "let placed = select * from \"host.placed\"; emit placed subject \"path\" key(\"spec\");";
const MISSPELLED: &str =
    "let functions = selekt * from \"function\"; emit functions subject \"file\" key(\"span\");";

/// What most rows state: the functions rule over a directory holding one file, walked, with nothing kept, no budget and nothing hosted.
const OPTIONAL_POSITIVE: &str =
    "let names = select * from \"identifier\"; emit names subject file key(span);";
const OPTIONAL_ABSENCE: &str = "let names = select * from \"identifier\" where name = \"missing\"; let absent = antijoin(left=\"syntax-tree\",right=names,left-on=[\"file\"],right-on=[\"file\"]); emit absent subject file key(span);";
const RAW_FORMAT_POSITIVE: &str = "let segments = select * from \"raw-format-segment\" where outcome = \"parsed\"; emit segments subject file key(span);";

/// A syntactic literal outcome requires its advertised optional capability.
const LITERAL_PARSE_POSITIVE: &str = "let parsed = select * from \"literal-expression-parse\" where outcome = \"parsed\"; emit parsed subject file key(span);";

const LINTED: Given = Given {
    query: FUNCTIONS,
    kept: false,
    root: Root::Directory,
    bytes: None,
    hosted: Hosting::None,
    corpus: Reads::Walked,
    linter: Linting::Fresh,
};

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("literal_parse_contract_changes_cannot_replay_cached_absence"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Capabilities,
            query: LITERAL_PARSE_POSITIVE,
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 0)),
    },
    Case {
        name: CaseName("raw_format_contract_changes_cannot_replay_cached_absence"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Capabilities,
            query: RAW_FORMAT_POSITIVE,
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 0)),
    },
    Case {
        name: CaseName("optional_contract_changes_cannot_replay_cached_absence"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Capabilities,
            query: OPTIONAL_ABSENCE,
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 0)),
    },
    Case {
        name: CaseName("optional_contract_changes_cannot_replay_cached_findings"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Capabilities,
            query: OPTIONAL_POSITIVE,
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 0)),
    },
    Case {
        name: CaseName("a_listed_corpus_reads_the_listed_files"),
        word: Word::Succeeds,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::Placed,
            corpus: Reads::Listed,
            ..LINTED
        },
        expect: Expect::Value((1, 1, true, 2)),
    },
    Case {
        name: CaseName("a_listed_corpus_is_the_boundary_of_what_the_rules_see"),
        word: Word::Succeeds,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::Placed,
            corpus: Reads::ListedNothing,
            ..LINTED
        },
        expect: Expect::Value((0, 0, true, 0)),
    },
    Case {
        name: CaseName("files_given_beside_the_disk_are_read_with_it"),
        word: Word::Succeeds,
        input: || Given {
            corpus: Reads::Beside,
            ..LINTED
        },
        expect: Expect::Value((0, 2, true, 0)),
    },
    Case {
        name: CaseName("lints_and_keeps_nothing_in_the_repository"),
        word: Word::Succeeds,
        input: || LINTED,
        expect: Expect::Value((0, 1, true, 0)),
    },
    Case {
        name: CaseName(
            "lints_and_keeps_its_store_where_the_caller_says_a_directory_no_run_made_yet",
        ),
        word: Word::Succeeds,
        input: || Given {
            kept: true,
            ..LINTED
        },
        expect: Expect::Value((0, 1, true, 0)),
    },
    Case {
        name: CaseName("a_file_the_host_says_was_placed_is_a_row_a_rule_reads"),
        word: Word::Succeeds,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::Placed,
            ..LINTED
        },
        expect: Expect::Value((1, 1, true, 2)),
    },
    Case {
        name: CaseName("files_given_as_text_are_read_instead_of_the_disk"),
        word: Word::Succeeds,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::Placed,
            corpus: Reads::Given,
            ..LINTED
        },
        expect: Expect::Value((1, 1, true, 2)),
    },
    Case {
        name: CaseName("a_shared_linter_gives_what_fresh_linters_give"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Shared,
            ..LINTED
        },
        expect: Expect::Value((1, 1, true, 2)),
    },
    Case {
        name: CaseName("a_row_with_fewer_cells_than_its_relation_has_columns_is_refused"),
        word: Word::Refuses,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::ShortRow,
            ..LINTED
        },
        expect: Expect::Error(Refusal::Host),
    },
    Case {
        name: CaseName("a_cell_that_is_not_of_the_kind_of_its_column_is_refused"),
        word: Word::Refuses,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::WrongKind,
            ..LINTED
        },
        expect: Expect::Error(Refusal::Host),
    },
    Case {
        name: CaseName("two_relations_with_one_name_are_refused"),
        word: Word::Refuses,
        input: || Given {
            query: HOSTED,
            hosted: Hosting::Twice,
            ..LINTED
        },
        expect: Expect::Error(Refusal::Host),
    },
    Case {
        name: CaseName("a_rule_with_wrong_text_is_refused_with_its_id"),
        word: Word::Refuses,
        input: || Given {
            query: MISSPELLED,
            ..LINTED
        },
        expect: Expect::Error(Refusal::Rule),
    },
    Case {
        name: CaseName("a_repository_that_does_not_fit_the_budget_is_refused"),
        word: Word::Refuses,
        input: || Given {
            bytes: Some(1),
            ..LINTED
        },
        expect: Expect::Error(Refusal::TooLarge),
    },
    #[cfg(any(unix, windows))]
    Case {
        name: CaseName("a_root_that_is_not_text_is_refused"),
        word: Word::Refuses,
        input: || Given {
            root: Root::NotText,
            ..LINTED
        },
        expect: Expect::Error(Refusal::PathNotText),
    },
    Case {
        name: CaseName("a_root_that_is_not_there_fails_in_the_corpus"),
        word: Word::Fails,
        input: || Given {
            root: Root::Missing,
            ..LINTED
        },
        expect: Expect::Error(Refusal::Io),
    },
];

/// A path that is not text.
#[cfg(unix)]
fn not_text() -> std::path::PathBuf {
    use std::os::unix::ffi::OsStringExt as _;
    std::ffi::OsString::from_vec(vec![b'/', 0xff, 0xfe]).into()
}

/// A path that is not text: a lone surrogate, which no UTF-8 holds.
#[cfg(windows)]
fn not_text() -> std::path::PathBuf {
    use std::os::windows::ffi::OsStringExt as _;
    std::ffi::OsString::from_wide(&[u16::from(b'/'), 0xD800]).into()
}

/// The relation of the placed files the host declares: the file that is on disk, or the one given as text.
fn hosted(hosting: Hosting, given_as_text: bool) -> Vec<HostRelation> {
    let file = if given_as_text {
        "from-memory.txt"
    } else {
        "notes.txt"
    };
    let relation = |cells: Vec<Cell>| HostRelation {
        name: HostRelationName("host.placed".into()),
        columns: vec![
            HostColumn {
                name: HostColumnName("path".into()),
                kind: HostKind::File,
            },
            HostColumn {
                name: HostColumnName("spec".into()),
                kind: HostKind::Text,
            },
        ],
        rows: vec![HostRow(cells)],
    };
    let right = vec![
        Cell::File(RelativePath(file.into())),
        Cell::Text(HostText("docs".into())),
    ];
    match hosting {
        Hosting::None => Vec::new(),
        Hosting::Placed => vec![relation(right)],
        Hosting::ShortRow => vec![relation(vec![Cell::File(RelativePath(file.into()))])],
        Hosting::WrongKind => vec![relation(vec![
            Cell::Number(HostNumber(1)),
            Cell::Text(HostText("docs".into())),
        ])],
        Hosting::Twice => vec![relation(right.clone()), relation(right)],
    }
}

/// Lints a directory holding one file, with no language linked: the file is unread.
fn subject(given: Input) -> Result<Value, Failure> {
    if matches!(given.linter, Linting::Capabilities) {
        return Ok(capability_cache(given.query));
    }
    if matches!(given.linter, Linting::Shared) {
        return Ok(shared());
    }
    let root = tempfile::tempdir().expect("a temporary directory");
    let elsewhere = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(root.path().join("notes.txt"), "text").expect("a file");
    let id = RuleId::try_from("rule").expect("a rule id");
    let request = LintRequest {
        source_language: None,
        root: match given.root {
            Root::Directory => root.path().to_path_buf(),
            Root::Missing => root.path().join("not-there"),
            #[cfg(any(unix, windows))]
            Root::NotText => not_text(),
        },
        rules: BTreeMap::from([(
            id,
            Rule {
                observations: Vec::new(),
                query: crate::resources::query::RyoQlQuery::try_from(given.query).expect("a query"),
                applicability: crate::resources::linter::RuleApplicability::Always,
                parameters: BTreeMap::new(),
                include: Vec::new(),
                exclude: Vec::new(),
            },
        )]),
        files: Vec::new(),
        hosted: hosted(given.hosted, matches!(given.corpus, Reads::Given)),
        corpus: match given.corpus {
            Reads::Walked | Reads::Beside => Corpus::Walked,
            Reads::Listed => Corpus::Listed(vec![RelativePath("notes.txt".into())]),
            Reads::ListedNothing => Corpus::Listed(Vec::new()),
            Reads::Given => Corpus::Given(vec![CaseSource {
                executable: Default::default(),
                path: RelativePath("from-memory.txt".into()),
                text: SourceText("text".into()),
            }]),
        },
        beside: if matches!(given.corpus, Reads::Beside) {
            vec![CaseSource {
                executable: Default::default(),
                path: RelativePath("beside.txt".into()),
                text: SourceText("text".into()),
            }]
        } else {
            Vec::new()
        },
    };
    let memory = given
        .bytes
        .and_then(NonZeroU64::new)
        .map_or(MemoryBudget::Machine, MemoryBudget::Bytes);
    // A cache directory no run has made yet: the run spills there before it keeps its store, so it is made first.
    let cache = if given.kept {
        Cache::At(elsewhere.path().join("cache").join("ryo"))
    } else {
        Cache::None
    };
    let report = lint(
        Languages::default(),
        &crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
        memory,
        cache,
        request,
    )
    .map_err(|error| match error {
        LintError::Execution { reason } => Refusal::Execution(reason),
        LintError::ObservationBindings { .. } => Refusal::ObservationBindings,
        LintError::ObservationExecution { .. } => Refusal::ObservationExecution,
        LintError::SourceLanguage { .. } => Refusal::SourceLanguage,
        LintError::Rule { .. } => Refusal::Rule,
        LintError::Unrunnable { .. } => Refusal::Unrunnable,
        LintError::MissingParameter { .. } => Refusal::MissingParameter,
        LintError::TooLarge { .. } => Refusal::TooLarge,
        LintError::PathNotText { .. } => Refusal::PathNotText,
        LintError::Io { .. } => Refusal::Io,
        LintError::Provider { .. } => Refusal::Provider,
        LintError::DuplicateHostRelation { .. }
        | LintError::HostRowWidth { .. }
        | LintError::HostCellKind { .. } => Refusal::Host,
        LintError::MemoryBudgetExceeded { .. } => Refusal::MemoryBudgetExceeded,
    })?;
    let untouched = std::fs::read_dir(root.path()).is_ok_and(|entries| entries.count() == 1)
        && (!given.kept || elsewhere.path().join("cache").join("ryo").is_dir());
    let columns = report
        .findings
        .first()
        .map_or(0, |finding| finding.columns.len());
    Ok((
        report.findings.len(),
        report.unread.len(),
        untouched,
        columns,
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

/// The two calls the shared-linter row tells apart: one rule id, two queries, two declarations of the host, two sets of globs and
/// two sets of files given as text.
fn shared_calls(root: &std::path::Path) -> [LintRequest; 2] {
    let rule = |query: &str, include: &str| Rule {
        observations: Vec::new(),
        query: crate::resources::query::RyoQlQuery::try_from(query).expect("a query"),
        applicability: crate::resources::linter::RuleApplicability::Always,
        parameters: BTreeMap::new(),
        include: vec![
            crate::resources::linter::lint::types::Glob::try_from(include).expect("a glob"),
        ],
        exclude: Vec::new(),
    };
    let placed = |file: &str, spec: &str| HostRelation {
        name: HostRelationName("host.placed".into()),
        columns: vec![
            HostColumn {
                name: HostColumnName("path".into()),
                kind: HostKind::File,
            },
            HostColumn {
                name: HostColumnName("spec".into()),
                kind: HostKind::Text,
            },
        ],
        rows: vec![HostRow(vec![
            Cell::File(RelativePath(file.into())),
            Cell::Text(HostText(spec.into())),
        ])],
    };
    let given = |file: &str| {
        Corpus::Given(vec![CaseSource {
            executable: Default::default(),
            path: RelativePath(file.into()),
            text: SourceText("text".into()),
        }])
    };
    let request = |query: &str, include: &str, file: &str, spec: &str| LintRequest {
        source_language: None,
        root: root.to_path_buf(),
        rules: BTreeMap::from([(
            RuleId::try_from("rule").expect("a rule id"),
            rule(query, include),
        )]),
        files: Vec::new(),
        hosted: vec![placed(file, spec)],
        corpus: given(file),
        beside: Vec::new(),
    };
    let other = "let placed = select * from \"host.placed\"; emit placed subject \"path\" key(\"path\", \"spec\");";
    [
        request(HOSTED, "*.txt", "from-memory.txt", "docs"),
        request(other, "*.md", "other.md", "notes"),
    ]
}

/// Two different calls on one linter, one after the other, interleaved and in parallel, give exactly what the same calls give on a
/// fresh linter each: nothing a call leaves in the linter reaches another. The value is the first call's, over a directory holding
/// one file.
fn shared() -> Value {
    let root = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(root.path().join("notes.txt"), "text").expect("a file");
    let linter = || {
        super::types::Linter::new(
            Languages::default(),
            crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            Cache::None,
        )
    };
    let fresh =
        |request: LintRequest| super::entry::lint_with(&linter(), request).expect("a report");
    let [first, second] = shared_calls(root.path());
    let expected = [fresh(first.clone()), fresh(second.clone())];
    assert_ne!(
        expected[0], expected[1],
        "the two calls tell different things"
    );
    let shared = linter();
    let call = |request: &LintRequest| {
        super::entry::lint_with(&shared, request.clone()).expect("a report")
    };
    for round in 0_u8..3 {
        assert_eq!(call(&first), expected[0], "round {round}, first");
        assert_eq!(call(&second), expected[1], "round {round}, second");
        assert_eq!(call(&second), expected[1], "round {round}, second again");
        assert_eq!(call(&first), expected[0], "round {round}, first again");
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0_usize..16)
            .map(|each| {
                let (request, want) = if each % 2_usize == 0 {
                    (&first, &expected[0])
                } else {
                    (&second, &expected[1])
                };
                let call = &call;
                scope.spawn(move || assert_eq!(&call(request), want, "parallel call {each}"))
            })
            .collect();
        for handle in handles {
            handle.join().expect("a call that did not panic");
        }
    });
    let untouched = std::fs::read_dir(root.path()).is_ok_and(|entries| entries.count() == 1);
    let columns = expected[0]
        .findings
        .first()
        .map_or(0, |finding| finding.columns.len());
    (
        expected[0].findings.len(),
        expected[0].unread.len(),
        untouched,
        columns,
    )
}

/// A provider with the same name/build whose advertised optional contract changes.
struct CapabilityProvider(
    &'static [ryo_relations::domain::relations::types::OptionalCapability],
    &'static str,
    &'static str,
);
impl ryo_language::inbound::use_cases::parse::types::ParseEngine for CapabilityProvider {
    fn begin_semantics(
        &self,
        _: &ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
        _: &ryo_language::domain::elements::types::Elements,
    ) -> Option<Box<dyn ryo_language::domain::semantics::interface::SemanticSession>> {
        None
    }
    fn capabilities(
        &self,
    ) -> &'static [ryo_relations::domain::relations::types::OptionalCapability] {
        self.0
    }
    fn configuration_identity(
        &self,
    ) -> ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity<'_> {
        ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity(self.1)
    }
    fn recognition(
        &self,
        _: &ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
    ) -> ryo_language::inbound::use_cases::parse::types::Recognition {
        ryo_language::inbound::use_cases::parse::types::Recognition::Reads
    }
    fn skips(&self) -> &'static [ryo_language::inbound::use_cases::parse::types::SkippedDirectory] {
        &[]
    }
    fn kind_count(&self) -> ryo_relations::domain::relations::types::Count {
        ryo_relations::domain::relations::types::Count(1)
    }
    fn kind_names(&self) -> ryo_language::inbound::use_cases::parse::types::KindNames {
        ryo_language::inbound::use_cases::parse::types::KindNames(vec![
            ryo_relations::domain::terms::types::NameText("root"),
        ])
    }
    fn unknown_reasons(&self) -> &'static [ryo_relations::domain::terms::types::UnknownName] {
        &[]
    }
    fn reader_kinds(&self) -> Vec<ryo_relations::domain::relations::types::KindBits> {
        vec![ryo_relations::domain::relations::types::KindBits(0)]
    }
    fn build_file(
        &self,
        request: ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
        table: &mut ryo_language::domain::elements::types::Elements,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<(), ryo_language::inbound::use_cases::parse::errors::ParseError> {
        use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder};
        use ryo_relations::domain::relations::types::{KindBits, Offset};
        let mut builder = table_builder(table, symbols, request.bytes);
        enter_node(&mut builder, KindBits(0), Offset(0));
        leave_node(&mut builder, Offset(1));
        ryo_language::domain::completion::service::name_kind(
            &mut ryo_language::domain::completion::service::completion(
                table,
                symbols,
                request.bytes,
                request.file,
            ),
            KindBits(0),
            ryo_relations::domain::terms::types::NameText("root"),
        );
        Ok(())
    }
    fn baseline_file(
        &self,
        _: &ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
    ) -> Result<(), ryo_language::inbound::use_cases::parse::errors::ParseError> {
        Ok(())
    }
    fn read_file(
        &self,
        request: ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
        table: &mut ryo_language::domain::elements::types::Elements,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        wanted: &[ryo_relations::domain::relations::types::Relation],
        out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
    ) -> Result<(), ryo_language::inbound::use_cases::parse::errors::ParseError> {
        let source = request.bytes;
        let file = request.file;
        self.build_file(request, table, symbols)?;
        self.walk_file(table, symbols, source, file, wanted, out);
        Ok(())
    }
    fn walk_file(
        &self,
        _: &mut ryo_language::domain::elements::types::Elements,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        source: ryo_language::domain::symbols::types::ByteSlice<'_>,
        file: ryo_relations::domain::relations::types::FileId,
        wanted: &[ryo_relations::domain::relations::types::Relation],
        out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
    ) {
        use ryo_relations::domain::relations::{
            collections::relations::identifier::item::IdentifierRow,
            types::{
                Missing, Offset, OptionalCapability, Position, Relation, ReportText, Rows, Span,
            },
        };
        use ryo_relations::domain::terms::types::NameText;
        if wanted.contains(&Relation::Identifier) {
            if self.0.contains(&OptionalCapability::Identifier) {
                out.write(IdentifierRow {
                    id: ryo_relations::domain::relations::service::node_id(file, Position(0)),
                    name: ryo_language::domain::symbols::service::intern_derived(
                        symbols,
                        source,
                        NameText(self.2),
                    ),
                    file,
                    span: Span {
                        start: Offset(0),
                        end: Offset(1),
                    },
                });
            } else {
                <dyn ryo_relations::domain::relations::types::RowWriter as Rows<IdentifierRow>>::partial_at(out,file,ReportText("unadvertised fixture capability"),Missing::Any);
            }
        }
        if wanted.contains(&Relation::LiteralExpressionParse) {
            literal_parse_capability_row(
                self.0.contains(&OptionalCapability::LiteralExpressionParse),
                source,
                file,
                symbols,
                out,
            );
        }
        if wanted.contains(&Relation::RawFormatSegment) {
            use ryo_relations::domain::relations::collections::relations::raw_format_segment::item::RawFormatSegmentRow;
            if self.0.contains(&OptionalCapability::RawFormatSegment) {
                let intern =
                    |interned_symbols: &mut ryo_language::domain::symbols::types::Symbols,
                     text: &str| {
                        ryo_language::domain::symbols::service::intern_derived(
                            interned_symbols,
                            source,
                            NameText(text),
                        )
                    };
                let id = ryo_relations::domain::relations::service::node_id(file, Position(0));
                out.write(RawFormatSegmentRow {
                    id,
                    segment: id,
                    segment_ordinal: ryo_relations::domain::relations::types::Integer(0),
                    dialect: intern(symbols, "percent-string"),
                    outcome: intern(symbols, "parsed"),
                    error_category: intern(symbols, "none"),
                    error_character: intern(symbols, ""),
                    error_index: ryo_relations::domain::relations::types::Integer(0),
                    positional_consumption: ryo_relations::domain::relations::types::Integer(0),
                    mapping_present: ryo_relations::domain::relations::types::Flag::Lowered,
                    dynamic_width: ryo_relations::domain::relations::types::Flag::Lowered,
                    dynamic_precision: ryo_relations::domain::relations::types::Flag::Lowered,
                    file,
                    span: Span {
                        start: Offset(0),
                        end: Offset(1),
                    },
                });
            } else {
                <dyn ryo_relations::domain::relations::types::RowWriter as Rows<
                    RawFormatSegmentRow,
                >>::partial_at(
                    out,
                    file,
                    ReportText("unadvertised raw format capability"),
                    Missing::Any,
                );
            }
        }
    }
    fn absence_of(
        &self,
        relation: ryo_relations::domain::relations::types::Relation,
    ) -> Option<ryo_relations::domain::answers::types::AbsenceCause> {
        use ryo_relations::domain::{
            answers::types::AbsenceCause,
            relations::types::{OptionalCapability, Relation},
        };
        ((relation == Relation::Identifier && !self.0.contains(&OptionalCapability::Identifier))
            || (relation == Relation::RawFormatSegment
                && !self.0.contains(&OptionalCapability::RawFormatSegment))
            || (relation == Relation::LiteralExpressionParse
                && !self.0.contains(&OptionalCapability::LiteralExpressionParse)))
        .then_some(AbsenceCause::NotYetRead)
    }
}

/// Emit or explicitly refuse the optional literal fact in the persistent-identity fixture.
fn literal_parse_capability_row(
    available: bool,
    source: ryo_language::domain::symbols::types::ByteSlice<'_>,
    file: ryo_relations::domain::relations::types::FileId,
    symbols: &mut ryo_language::domain::symbols::types::Symbols,
    out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
) {
    use ryo_relations::domain::relations::{
        collections::relations::literal_expression_parse::item::LiteralExpressionParseRow,
        types::{Missing, Offset, Position, ReportText, Rows, Span},
    };
    use ryo_relations::domain::terms::types::NameText;
    if available {
        let dialect = ryo_language::domain::symbols::service::intern_derived(
            symbols,
            source,
            NameText("python-string-annotation"),
        );
        let outcome = ryo_language::domain::symbols::service::intern_derived(
            symbols,
            source,
            NameText("parsed"),
        );
        out.write(LiteralExpressionParseRow {
            id: ryo_relations::domain::relations::service::node_id(file, Position(0)),
            dialect,
            outcome,
            file,
            span: Span {
                start: Offset(0),
                end: Offset(1),
            },
        });
    } else {
        <dyn ryo_relations::domain::relations::types::RowWriter as Rows<
            LiteralExpressionParseRow,
        >>::partial_at(
            out,
            file,
            ReportText("unadvertised literal parse capability"),
            Missing::Any,
        );
    }
}

/// Same-process promises must not replay findings retained by a different advertised contract.
#[expect(
    clippy::too_many_lines,
    reason = "this regression keeps cold, warm, and changed-capability cache paths in one scenario"
)]
fn capability_cache(query: &str) -> Value {
    use ryo_relations::domain::relations::types::OptionalCapability;
    let advertised_capabilities: &'static [OptionalCapability] = if query == RAW_FORMAT_POSITIVE {
        &[
            OptionalCapability::RawFormatSegment,
            OptionalCapability::RawFormatField,
        ]
    } else if query == LITERAL_PARSE_POSITIVE {
        &[OptionalCapability::LiteralExpressionParse]
    } else {
        &[OptionalCapability::Identifier]
    };
    let root = tempfile::tempdir().expect("corpus");
    let store = tempfile::tempdir().expect("store");
    std::fs::write(root.path().join("case.txt"), "x").expect("source");
    let language = |caps, configuration, name| {
        crate::domain::language::types::Language::from_parts(
            crate::domain::language::types::LanguageName("fixture".into()),
            crate::domain::language::types::LanguageBuild("same-build".into()),
            std::sync::Arc::new(CapabilityProvider(caps, configuration, name)),
            std::sync::Arc::new(
                ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
            ),
        )
    };
    let engine = |caps, configuration, name, cache| {
        super::types::Linter::new(
            Languages(vec![language(caps, configuration, name)]),
            crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            cache,
        )
    };
    let request = LintRequest {
        source_language: None,
        root: root.path().to_path_buf(),
        rules: BTreeMap::from([(
            RuleId::try_from("optional").expect("rule"),
            Rule {
                observations: Vec::new(),
                query: crate::resources::query::RyoQlQuery::try_from(query).expect("query"),
                applicability: crate::resources::linter::RuleApplicability::Always,
                parameters: BTreeMap::new(),
                include: vec![super::types::Glob::try_from("*.txt").expect("glob")],
                exclude: vec![],
            },
        )]),
        files: vec![],
        hosted: vec![],
        corpus: Corpus::Listed(vec![RelativePath("case.txt".into())]),
        beside: vec![],
    };
    let forward = super::connections::build(&[language(
        &[
            OptionalCapability::Identifier,
            OptionalCapability::QualifiedName,
        ],
        "",
        "x",
    )]);
    let reverse = super::connections::build(&[language(
        &[
            OptionalCapability::QualifiedName,
            OptionalCapability::Identifier,
        ],
        "",
        "x",
    )]);
    assert_eq!(
        forward, reverse,
        "capability declaration order does not change identity"
    );

    let advertised = engine(
        advertised_capabilities,
        "",
        "x",
        Cache::At(store.path().to_path_buf()),
    );
    let first = super::entry::lint_with(&advertised, request.clone()).expect("advertised report");
    assert_eq!(
        first.findings.len(),
        1,
        "the cache must contain a real optional finding"
    );
    let mut selected = request.clone();
    selected.source_language = Some(crate::domain::language::types::LanguageName(
        "fixture".into(),
    ));
    let explicit =
        super::entry::lint_with(&advertised, selected.clone()).expect("explicit refusal report");
    assert!(
        explicit.findings.is_empty(),
        "cached path-recognized findings cannot bypass explicit opt-in"
    );
    assert_eq!(
        explicit.unread.len(),
        1,
        "a non-opting provider must refuse explicit selection"
    );
    let fresh_explicit = super::entry::lint_with(
        &engine(advertised_capabilities, "", "x", Cache::None),
        selected.clone(),
    )
    .expect("fresh explicit refusal");
    assert_eq!(
        explicit, fresh_explicit,
        "source selection must be in cache identity"
    );
    assert_eq!(
        super::entry::lint_with(&advertised, request.clone()).expect("ordinary selection restored"),
        first
    );
    selected.source_language = Some(crate::domain::language::types::LanguageName(
        "missing-provider".into(),
    ));
    assert!(matches!(
        super::entry::lint_with(&advertised, selected),
        Err(super::errors::LintError::SourceLanguage { .. })
    ));

    let ambiguous = super::types::Linter::new(
        Languages(vec![
            language(advertised_capabilities, "", "x"),
            language(advertised_capabilities, "", "x"),
        ]),
        crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        Cache::None,
    );
    let mut ambiguous_request = request.clone();
    ambiguous_request.source_language = Some(crate::domain::language::types::LanguageName(
        "fixture".into(),
    ));
    assert!(matches!(
        super::entry::lint_with(&ambiguous, ambiguous_request),
        Err(super::errors::LintError::SourceLanguage { .. })
    ));

    let absent = engine(&[], "", "x", Cache::At(store.path().to_path_buf()));
    let replay = super::entry::lint_with(&absent, request.clone()).expect("unadvertised report");
    let fresh = super::entry::lint_with(&engine(&[], "", "x", Cache::None), request.clone())
        .expect("fresh unadvertised report");
    assert_eq!(
        replay, fresh,
        "retained data must not resurrect unadvertised findings"
    );
    assert!(
        fresh.findings.is_empty(),
        "unadvertised means unavailable, not a positive"
    );
    assert!(
        !fresh.unknowns.is_empty(),
        "unavailability remains observable"
    );
    let restored = engine(
        advertised_capabilities,
        "",
        "x",
        Cache::At(store.path().to_path_buf()),
    );
    assert_eq!(
        super::entry::lint_with(&restored, request.clone()).expect("restored report"),
        first,
        "prior unavailable state cannot poison newly advertised capability results"
    );

    if query == OPTIONAL_POSITIVE {
        let target_310 = engine(
            &[OptionalCapability::Identifier],
            "python-target=3.10",
            "x",
            Cache::At(store.path().to_path_buf()),
        );
        let before_target_change =
            super::entry::lint_with(&target_310, request.clone()).expect("Python 3.10 result");
        let target_314 = engine(
            &[OptionalCapability::Identifier],
            "python-target=3.14",
            "y",
            Cache::At(store.path().to_path_buf()),
        );
        let after_target_change =
            super::entry::lint_with(&target_314, request.clone()).expect("Python 3.14 result");
        let fresh_314 = super::entry::lint_with(
            &engine(
                &[OptionalCapability::Identifier],
                "python-target=3.14",
                "y",
                Cache::None,
            ),
            request.clone(),
        )
        .expect("fresh Python 3.14 result");
        assert_ne!(before_target_change, after_target_change);
        assert_eq!(after_target_change, fresh_314);
        let restored_310 = super::entry::lint_with(
            &engine(
                &[OptionalCapability::Identifier],
                "python-target=3.10",
                "x",
                Cache::At(store.path().to_path_buf()),
            ),
            request.clone(),
        )
        .expect("restored Python 3.10 result");
        assert_eq!(restored_310, before_target_change);
        let stable_identity = super::connections::build(&[language(
            &[OptionalCapability::Identifier],
            "python-target=3.14",
            "y",
        )]);
        assert_eq!(
            stable_identity,
            super::connections::build(&[language(
                &[OptionalCapability::Identifier],
                "python-target=3.14",
                "y",
            )]),
            "unchanged parser settings keep the same cache identity"
        );
        assert_ne!(
            stable_identity,
            super::connections::build(&[language(
                &[OptionalCapability::Identifier],
                "python-target=3.10",
                "x",
            )]),
            "changed parser settings change cache identity"
        );
    }
    (1, 0, true, 0)
}

mod executable;

mod host_identity;

mod observations;
