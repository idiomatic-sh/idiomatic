//! The test data of `linter lint` that fixes the request a row states: what the caller states and the request it makes.

use std::collections::BTreeMap;
use std::num::NonZeroU64;

use super::outcome::Outcome;
use super::queries::FUNCTIONS;
use crate::domain::capacity::types::MemoryBudget;
use crate::domain::cell::types::{Cell, HostNumber, HostText};
use crate::domain::file::types::RelativePath;
use crate::domain::host::types::{HostColumn, HostColumnName, HostKind, HostRelationName};
use crate::resources::linter::lint::errors::LintError;
use crate::resources::linter::lint::types::Cache;
use crate::resources::linter::lint::types::CaseSource;
use crate::resources::linter::lint::types::Corpus;
use crate::resources::linter::lint::types::HostRelation;
use crate::resources::linter::lint::types::HostRow;
use crate::resources::linter::lint::types::LintRequest;
use crate::resources::linter::lint::types::Rule;
use crate::resources::linter::lint::types::RuleId;
use crate::resources::linter::lint::types::SourceText;

/// Where the repository is.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Root {
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
pub(crate) enum Linting {
    /// The one call, to a linter of its own.
    Fresh,
    /// Two different calls to one linter, one after the other, interleaved and in parallel, each checked against what the same
    /// call gives on a fresh linter: the value is the first call's.
    Shared,
    /// Same-binary optional capability changes invalidate retained observations.
    Capabilities,
    /// A scenario of the fixtures, over a linter and a repository of its own.
    Scenario(fn() -> Result<Outcome, LintError>),
}

/// What the caller states: the rule's text, whether the run keeps what it leaves in a directory of the caller's, where the repository
/// is, the most memory the run may use in bytes (none: the machine's), whether the file it holds is said to be placed, and whether the
/// files are given as text, and which linter the calls go to.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Given {
    pub(crate) query: &'static str,
    pub(crate) kept: bool,
    pub(crate) root: Root,
    pub(crate) bytes: Option<u64>,
    pub(crate) hosted: Hosting,
    pub(crate) corpus: Reads,
    pub(crate) linter: Linting,
}

/// Which files the run is told to read.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Reads {
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
pub(crate) enum Hosting {
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

/// The request a row states, over the repository at `root`.
pub(crate) fn request(given: Given, root: &std::path::Path) -> LintRequest {
    let id = RuleId::try_from("rule").expect("a rule id");
    LintRequest {
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        source_language: crate::domain::language::types::SourceLanguage::Recognized,
        root: match given.root {
            Root::Directory => root.to_path_buf(),
            Root::Missing => root.join("not-there"),
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
    }
}

/// What most rows state: the functions rule over a directory holding one file, walked, with nothing kept, no budget and nothing hosted.
pub(crate) const LINTED: Given = Given {
    query: FUNCTIONS,
    kept: false,
    root: Root::Directory,
    bytes: None,
    hosted: Hosting::None,
    corpus: Reads::Walked,
    linter: Linting::Fresh,
};

/// The most memory a row lets the run use: its bytes, or the machine's when it states none.
pub(crate) fn memory(given: Given) -> MemoryBudget {
    given
        .bytes
        .and_then(NonZeroU64::new)
        .map_or(MemoryBudget::Machine, MemoryBudget::Bytes)
}

/// Where a row's run keeps its store, `elsewhere` when it keeps one: a cache directory no run has made yet, where the run spills
/// before it keeps its store, so it is made first.
pub(crate) fn cache(given: Given, elsewhere: &std::path::Path) -> Cache {
    if given.kept {
        Cache::At(elsewhere.join("cache").join("ryo"))
    } else {
        Cache::None
    }
}
