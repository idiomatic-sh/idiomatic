//! The scenario `primary_manifest_read_failure_retains_real_metadata_without_empty_project_source` of the cases of this file.

use super::*;

pub(super) fn primary_manifest_read_failure_retains_real_metadata_without_empty_project_source() -> Result<Value, Failure> {
    use ryo_values::domain::files::types::{
        CorpusEntry, CorpusInput, FileSize, InputReadFailure, Nanoseconds,
    };
    let corpus = Arc::new(CorpusFake::default());
    let service = FileAnalysis::new(
        corpus.clone(),
        Arc::new(LanguageFake::default()),
        Arc::new(DigestFake),
        Arc::new(MemoryFake::default()),
    );
    let entry = CorpusEntry {
        executable: Default::default(),
        path: FilePath("pkg/project.manifest.fake".into()),
        size: FileSize(42),
        modified: Nanoseconds(19),
    };
    let companion = CorpusEntry {
        path: FilePath("input.rs".into()),
        ..entry.clone()
    };
    let mut inputs = [
        CorpusInput::Available(entry.clone()),
        CorpusInput::Available(companion.clone()),
        CorpusInput::Unreadable {
            path: FilePath("missing.manifest.fake".into()),
            reason: InputReadFailure("not found".into()),
        },
    ];
    assert!(
        service.read_manifests(&mut inputs).unwrap().is_empty(),
        "a failed read never becomes empty manifest text"
    );
    assert_eq!(
        corpus.reads.load(Ordering::Relaxed),
        1,
        "only the available primary manifest is read; no consulted/source path is substituted"
    );
    let CorpusInput::ReadFailed {
        entry: observed,
        reason,
    } = &inputs[0]
    else {
        panic!("retain primary read outcome");
    };
    assert_eq!(observed, &entry);
    assert!(!reason.0.is_empty());
    assert!(matches!(&inputs[1], CorpusInput::Available(observed) if observed == &companion));
    assert!(matches!(&inputs[2], CorpusInput::Unreadable { .. }));
    Ok(1)
}
