//! What `linter lint` promises its caller, one row of `CASES` for each case it has.

use std::collections::BTreeMap;
use std::num::{NonZeroU64, NonZeroUsize};

use super::entry::lint;
use super::errors::LintError;
use super::fixtures::scenarios::{
    self, HOSTED, LITERAL_PARSE_POSITIVE, OPTIONAL_POSITIVE, Outcome, RAW_FORMAT_POSITIVE,
    capability_cache, shared,
};
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
    /// A scenario of the fixtures, over a linter and a repository of its own.
    Scenario(fn() -> Outcome),
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
type Value = Outcome;
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
const MISSPELLED: &str =
    "let functions = selekt * from \"function\"; emit functions subject \"file\" key(\"span\");";

/// What most rows state: the functions rule over a directory holding one file, walked, with nothing kept, no budget and nothing hosted.
const OPTIONAL_ABSENCE: &str = "let names = select * from \"identifier\" where name = \"missing\"; let absent = antijoin(left=\"syntax-tree\",right=names,left-on=[\"file\"],right-on=[\"file\"]); emit absent subject file key(span);";

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
        name: CaseName(
            "unchanged_bytes_and_a_given_timestamp_do_not_reuse_the_findings_of_an_old_permission",
        ),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(scenarios::permission_bytes),
            ..LINTED
        },
        expect: Expect::Value((1, 1, true, 1)),
    },
    Case {
        name: CaseName("an_unavailable_permission_is_neither_false_nor_a_proven_absence"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(scenarios::permission_unavailable),
            ..LINTED
        },
        expect: Expect::Value((1, 1, true, 1)),
    },
    Case {
        name: CaseName("changed_hosted_text_invalidates_retained_cross_file_findings"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(scenarios::hosted_text_identity),
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 1)),
    },
    Case {
        name: CaseName("a_later_rule_publishes_a_nonempty_observation_despite_zero_findings"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(scenarios::zero_finding_producer),
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 1)),
    },
    Case {
        name: CaseName("a_re_exported_publication_survives_cold_warm_replay_and_a_fresh_engine"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(scenarios::re_exported_publication),
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 2)),
    },
    Case {
        name: CaseName(
            "a_refused_and_a_truly_empty_producer_stay_distinct_on_cold_and_retained_replay",
        ),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(scenarios::refused_and_empty_producers),
            ..LINTED
        },
        expect: Expect::Value((0, 0, true, 0)),
    },
    Case {
        name: CaseName("a_selected_producer_with_no_query_is_never_a_complete_empty_input"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(scenarios::producer_without_query),
            ..LINTED
        },
        expect: Expect::Value((0, 0, true, 0)),
    },
    Case {
        name: CaseName("an_unbound_export_demands_no_reader_and_changes_no_ordinary_result"),
        word: Word::Succeeds,
        input: || Given {
            linter: Linting::Scenario(scenarios::unbound_export),
            ..LINTED
        },
        expect: Expect::Value((1, 0, true, 1)),
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
    if let Linting::Scenario(scenario) = given.linter {
        return Ok(scenario());
    }
    let root = tempfile::tempdir().expect("a temporary directory");
    let elsewhere = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(root.path().join("notes.txt"), "text").expect("a file");
    let id = RuleId::try_from("rule").expect("a rule id");
    let request = LintRequest {
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        source_language: crate::domain::language::types::SourceLanguage::Recognized,
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
        LintError::ProjectPath { .. } => Refusal::PathNotText,
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
