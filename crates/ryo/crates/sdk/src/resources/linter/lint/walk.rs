//! A request that says `Corpus::Walked` is listed here: the executor takes a closed inventory, so the SDK, which composes, walks the
//! root the way the engine always did (the repository's ignore files, the languages' skipped directories and the directory that holds the
//! store) and hands the executor the list.

use ryo_core::outbound::corpus::interface::CorpusPort as _;
use ryo_values::domain::files::types::{CorpusInput, CorpusListing};

use super::errors::LintError;
use super::types::{Cache, Corpus, LintReport, LintRequest, LinterLintEngine, Port};
use crate::domain::file::types::{ReadFailure, RelativePath, Unreadable};

/// `request` with its corpus listed: a walked corpus becomes the files below the root, as the walk finds them; every other corpus is
/// the caller's and stays as it is. With it, the files the walk found but could not read, and why: the report names them unread.
///
/// # Errors
///
/// `Io` of the corpus when the walk fails.
pub(super) fn listed(request: LintRequest, engine: &LinterLintEngine) -> Result<(LintRequest, Vec<(RelativePath, Box<str>)>), LintError> {
    listed_sized(request, engine).map(|(request, failed, _)| (request, failed))
}

/// `listed`, with the bytes of the files the walk found: what a memory budget is held against before any file is read.
///
/// # Errors
///
/// `Io` of the corpus when the walk fails.
pub(super) fn listed_sized(mut request: LintRequest, engine: &LinterLintEngine) -> Result<(LintRequest, Vec<(RelativePath, Box<str>)>, u64), LintError> {
    if !matches!(request.corpus, Corpus::Walked) {
        return Ok((request, Vec::new(), 0));
    }
    let mut skipped = engine.skipped.clone();
    if let Cache::At(directory) = &engine.cache
        && let Some(skip) = store_skip(directory, &request.root)
        && let Some(skip) = skip.to_str()
    {
        skipped.push(skip.into());
    }
    let walk = ryo_core::outbound::corpus::factory::ignore(
        ryo_language::inbound::use_cases::project::types::RootPath(request.root.to_string_lossy().into()),
        CorpusListing::Walked,
        Vec::new(),
        ryo_core::outbound::corpus::types::SkippedDirectories(skipped),
    );
    let inventory = walk.inventory().map_err(|error| LintError::Io { what: Port::Corpus, reason: error.to_string() })?;
    let mut paths = Vec::new();
    let mut failed = Vec::new();
    let mut size = 0_u64;
    for input in inventory.0 {
        match input {
            CorpusInput::Available(entry) => {
                size += entry.size.0;
                paths.push(RelativePath(entry.path.0.clone()));
            }
            CorpusInput::ReadFailed { entry, reason } => {
                paths.push(RelativePath(entry.path.0.clone()));
                failed.push((RelativePath(entry.path.0.clone()), reason.0));
            }
            CorpusInput::Unreadable { .. } => {}
        }
    }
    request.corpus = Corpus::Listed(paths);
    Ok((request, failed, size))
}

/// Where `path` is, written from the current directory without `.` or `..` components: the same place whatever its spelling.
fn normalized(path: &std::path::Path) -> Option<std::path::PathBuf> {
    let Ok(absolute) = std::path::absolute(path) else {
        return None;
    };
    let mut parts = std::path::PathBuf::new();
    for part in absolute.components() {
        match part {
            std::path::Component::ParentDir => {
                let _popped = parts.pop();
            }
            std::path::Component::CurDir => {}
            kept @ (std::path::Component::Prefix(_)
            | std::path::Component::RootDir
            | std::path::Component::Normal(_)) => parts.push(kept),
        }
    }
    Some(parts)
}

/// The walk's skip for the directory that holds the store: `./` and the path beneath `root`, however either is spelled, when the
/// store is kept beneath the root; none when it is kept elsewhere. A path, not a name: only that directory is skipped.
pub(super) fn store_skip(
    store: &std::path::Path,
    root: &std::path::Path,
) -> Option<std::path::PathBuf> {
    let (Some(store), Some(root)) = (normalized(store), normalized(root)) else {
        return None;
    };
    let Ok(beneath) = store.strip_prefix(root) else {
        return None;
    };
    if beneath.as_os_str().is_empty() {
        return None;
    }
    // Joined by `/` on every platform, as the walk reads every path it skips.
    let mut skip = std::path::Component::CurDir
        .as_os_str()
        .to_string_lossy()
        .into_owned();
    for part in beneath.components() {
        skip.push(SKIP_SEPARATOR);
        skip.push_str(&part.as_os_str().to_string_lossy());
    }
    Some(std::path::PathBuf::from(skip))
}

/// The separator of a skipped path, as the walk reads it.
const SKIP_SEPARATOR: char = '/';

/// `report` with the files the walk could not read named unread, beside those the executor reported, in path order.
pub(super) fn naming_unread(mut report: LintReport, failed: Vec<(RelativePath, Box<str>)>) -> LintReport {
    for (path, reason) in failed {
        if !report.unread.iter().any(|each| each.path == path) {
            report.unread.push(super::executor::unread(path, Unreadable::Failed(ReadFailure(reason))));
        }
    }
    report.unread.sort_by(|left, right| left.path.cmp(&right.path));
    report
}
