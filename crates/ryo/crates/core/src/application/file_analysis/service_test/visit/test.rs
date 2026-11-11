//! Visits of files of a fake corpus, the way a lint makes them, for the cases of `file_analysis`.
use super::super::super::errors::FileAnalysisError;
use super::super::fixtures::visits::visited;
use super::super::fixtures::visits::visited_as;
use crate::outbound::language::fixtures::fake::LanguageFake;
use std::sync::Arc;
use std::sync::atomic::Ordering;
/// Visit a real fake file whose injected language emits an invalid node reference.
pub(in super::super) fn invalid_provider() -> Result<usize, FileAnalysisError> {
    let (worker, language) = visited(
        &["invalid-provider.rs"],
        LanguageFake {
            out_of_range: true,
            ..LanguageFake::default()
        },
    )?;
    assert_eq!(language.parses.load(Ordering::Relaxed), 1);
    worker.failure.map_or(Ok(0), Err)
}

/// Visit a source file, a manifest the language reads and a stray file no language reads: whether the one file left unread is the stray one.
pub(in super::super) fn manifest_and_stray() -> Result<usize, FileAnalysisError> {
    let (worker, _) = visited(
        &["a.rs", "project.manifest.fake", "stray.xyz"],
        LanguageFake::default(),
    )?;
    Ok(usize::from(
        worker.failure.is_none()
            && worker.tables.unread.len() == 1
            && worker
                .tables
                .unread
                .iter()
                .all(|file| &*file.path.0 == "stray.xyz"),
    ))
}

/// How many times a run that keeps its findings digests two files read over a store that keeps nothing (tens), and over one that
/// keeps (ones, at least one): none, then some.
pub(in super::super) fn digested() -> Result<usize, FileAnalysisError> {
    let mut counts = Vec::new();
    for keeping in [
        crate::domain::visit::types::Keeping::Discards,
        crate::domain::visit::types::Keeping::Keeps,
    ] {
        let digest = Arc::new(
            ryo_execution_runtime::outbound::digest::fixtures::fake::DigestCounting::default(),
        );
        let (worker, _) = visited_as(
            &["a.rs", "b.rs"],
            LanguageFake::default(),
            &crate::domain::render::types::Answering::Findings,
            keeping,
            Arc::<ryo_execution_runtime::outbound::digest::fixtures::fake::DigestCounting>::clone(
                &digest,
            ),
        )?;
        assert!(worker.failure.is_none(), "the files are read");
        assert_eq!(worker.tables.fresh.len(), 2, "and their findings recorded");
        counts.push(digest.digests.load(Ordering::Relaxed));
    }
    Ok(counts.first().copied().unwrap_or(1) * 10
        + usize::from(counts.get(1).is_some_and(|each| *each > 0)))
}
