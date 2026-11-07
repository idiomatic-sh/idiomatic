//! The scenario `discovery_failure_runs_without_parser_bytes_or_permission_invention` of the cases of this file.

use super::*;

pub(super) fn discovery_failure_runs_without_parser_bytes_or_permission_invention() -> Result<Value, Failure> {
    use crate::domain::{render::types::Answering, visit::types::Keeping};
    use ryo_values::domain::files::types::{CorpusInput, InputReadFailure};
    let corpus = Arc::new(CorpusFake::default());
    let input = CorpusInput::Unreadable {
        path: FilePath("missing.py".into()),
        reason: InputReadFailure("not found".into()),
    };
    let (worker, language) = visited_inputs(
        corpus.clone(),
        &[input],
        &[
            Relation::FilesystemFile,
            Relation::FilesystemReadFailure,
            Relation::FilesystemExecutable,
            Relation::FilesystemLineCount,
            Relation::Function,
        ],
        LanguageFake::default(),
        &Answering::Findings,
        Keeping::Keeps,
        Arc::new(DigestFake),
        FactsFake::default().worker().unwrap(),
        None,
        1,
    )
    .unwrap();
    assert!(worker.failure.is_none());
    assert_eq!(
        corpus.reads.load(Ordering::Relaxed),
        0,
        "there is no fabricated entry to read"
    );
    assert_eq!(language.parses.load(Ordering::Relaxed), 0);
    assert!(worker.tables.bytes.0.is_empty());
    for relation in [
        Relation::FilesystemFile,
        Relation::FilesystemExecutable,
        Relation::FilesystemLineCount,
        Relation::Function,
    ] {
        assert!(
            worker
                .tables
                .partials
                .iter()
                .any(|partial| partial.relation == relation)
        );
    }
    assert!(
        !worker
            .tables
            .partials
            .iter()
            .any(|partial| partial.relation == Relation::FilesystemReadFailure)
    );
    assert!(worker.tables.fresh.is_empty());
    assert_eq!(worker.tables.unrecorded, [FilePath("missing.py".into())]);
    Ok(1)
}
