//! The tables the contract keeps in a store and the runs that keep them: samples of what a lint stores, small enough to read.

use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::reasons::types::ToolMessage;
use ryo_relations::domain::relations::types::Offset;
use ryo_relations::domain::relations::types::Span;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::descriptions::types::LanguageName;
use ryo_values::domain::descriptions::types::RelationName;
use ryo_values::domain::files::types::FileSize;
use ryo_values::domain::files::types::Nanoseconds;
use ryo_values::domain::records::types::GoverningRules;
use ryo_values::domain::records::types::Unreadable;
use ryo_values::domain::row_texts::types::Cell;
use ryo_values::domain::row_texts::types::PlacedColumn;
use ryo_values::domain::row_texts::types::PlacedLine;
use ryo_values::domain::row_texts::types::PlacedRule;
use ryo_values::domain::runs::types::ManifestRefused;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stored_facts::types::FileNotes;
use ryo_values::domain::stored_facts::types::LanguageNumber;
use ryo_values::domain::stored_facts::types::NoteBytes;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_facts::types::PartialCause;
use ryo_values::domain::stored_facts::types::PartialKind;
use ryo_values::domain::stored_facts::types::PartialScope;
use ryo_values::domain::stored_facts::types::ReadFailure;
use ryo_values::domain::stored_facts::types::StoredPartial;
use ryo_values::domain::stored_findings::types::FindingMade;
use ryo_values::domain::stored_findings::types::FindingPlace;
use ryo_values::domain::stored_findings::types::StoredCell;
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use ryo_values::domain::stored_findings::types::StoredFinding;
use ryo_values::domain::stored_findings::types::StoredPlace;
use ryo_values::domain::stored_findings::types::StoredUnknown;
use ryo_values::domain::stored_findings::types::StoredUnread;
use ryo_values::domain::stored_findings::types::StoredValue;
use ryo_values::domain::stored_findings::types::UnknownText;
use ryo_values::domain::stores::types::Crossing;
use ryo_values::domain::stores::types::FileAgain;
use ryo_values::domain::stores::types::KeptRows;
use ryo_values::domain::stores::types::Stored;
use ryo_values::domain::stores::types::StoredCovered;
use ryo_values::domain::stores::types::StoredFile;
use ryo_values::domain::stores::types::StoredFrontier;
use ryo_values::domain::stores::types::StoredHanded;
use ryo_values::domain::stores::types::StoredNotes;
use ryo_values::domain::stores::types::StoredRule;
use ryo_values::domain::stores::types::StoredText;
use ryo_values::domain::stores::types::StoredWords;

/// Registers, in `handed.proofs`, exactly the proof of every frontier row: a store keeps a frontier payload only under the exact
/// proof of its (file, bucket), as a run issues it, once. The fixtures replace frontier rows, so they register their proofs here.
pub fn register_proofs(handed: &mut StoredHanded) {
    let handed = &mut handed.0;
    handed.proofs.clear();
    for row in &handed.frontier {
        if !handed
            .proofs
            .iter()
            .any(|each| each.file == row.file && each.bucket == row.bucket)
        {
            handed
                .proofs
                .push(ryo_values::domain::stores::types::StoredFrontierProof {
                    file: row.file,
                    bucket: row.bucket,
                    proof: row.proof.clone(),
                });
        }
    }
    handed
        .proofs
        .sort_by_key(|each| (each.file.0, each.bucket.0));
}

/// The words one file handed, in bucket 3.
#[must_use]
pub fn words_of(file: u32, word: u8) -> StoredWords {
    StoredWords {
        file: StoredFileNumber(file),
        bucket: Bucket(3),
        proof: super::proof(),
        words: PackedWords(vec![word]),
    }
}

/// A file used again: its number before and now.
#[must_use]
pub fn again(was: u32, now: u32) -> FileAgain {
    FileAgain {
        was: StoredFileNumber(was),
        now: StoredFileNumber(now),
    }
}

/// The store a first run kept over the files `b` and `d`: their words are 10 and 11.
#[must_use]
pub fn over_b_and_d() -> Stored {
    let mut first = stored(1);
    first.files.truncate(2);
    first.handed.frontier = vec![words_of(0, 10), words_of(1, 11)];
    register_proofs(&mut first.handed);
    first
}

/// What a second run keeps having read the files `a` and `e` anew and used `b` and `d` again: the files are `a, b, d, e` now.
#[must_use]
pub fn over_a_b_d_and_e() -> (Stored, KeptRows) {
    let mut second = stored(2);
    if let Some(file) = second.files.first().cloned() {
        second.files = vec![file; 4];
    }
    second.handed.frontier = vec![words_of(0, 1), words_of(3, 13)];
    register_proofs(&mut second.handed);
    (
        second,
        KeptRows {
            staged: Vec::new(),
            unchanged: Vec::new(),
            file_again: vec![again(0, 1), again(1, 2)],
        },
    )
}

/// Tables with a row in every one, told apart by `tag`.
#[must_use]
pub fn stored(tag: u8) -> Stored {
    let path = |name: &str| FilePath(format!("src/{tag}/{name}").into());
    Stored {
        rules: vec![
            StoredRule {
                rule: RuleId(format!("rule-{tag}").into()),
                digest: Digest([tag; 16]),
                crossing: Crossing::Within,
            },
            StoredRule {
                rule: RuleId("across".into()),
                digest: Digest([tag.wrapping_add(1); 16]),
                crossing: Crossing::Across,
            },
        ],
        files: vec![
            StoredFile {
                executable: Default::default(),
                path: path("a.rs"),
                digest: Digest([1; 16]),
                interface: Digest([2; 16]),
                noted: Digest([4; 16]),
                size: FileSize(u64::from(tag)),
                modified: Nanoseconds(7),
            },
            StoredFile {
                executable: Default::default(),
                path: path("b.md"),
                digest: Digest([3; 16]),
                interface: Digest([0; 16]),
                noted: Digest([0; 16]),
                size: FileSize(4),
                modified: Nanoseconds(8),
            },
        ],
        findings: vec![
            StoredFinding {
                rule: RuleId(format!("rule-{tag}").into()),
                made: FindingMade::InFile(StoredFileNumber(0)),
                place: FindingPlace::At(StoredPlace {
                    line: PlacedLine(3),
                    column: PlacedColumn(5),
                }),
                subject: StoredCell::Value(Cell::File(path("a.rs").0)),
                key: vec![StoredCell::Value(Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(4),
                }))],
                values: vec![StoredValue {
                    column: ColumnName("name".into()),
                    value: StoredCell::Value(Cell::Text("main".into())),
                }],
            },
            StoredFinding {
                rule: RuleId("across".into()),
                made: FindingMade::AcrossFiles,
                place: FindingPlace::Unplaced,
                subject: StoredCell::Value(Cell::File("Cargo.toml".into())),
                key: vec![
                    StoredCell::Value(Cell::Text("a".into())),
                    StoredCell::Nothing,
                ],
                values: Vec::new(),
            },
        ],
        unread: vec![StoredUnread {
            file: StoredFileNumber(1),
            reason: Unreadable::NoLanguage,
        }],
        refused: vec![ManifestRefused {
            language: LanguageName("rust".into()),
            path: FilePath("Cargo.toml".into()),
            reason: ToolMessage("broken".into()),
        }],
        unknowns: vec![StoredUnknown {
            rule: RuleId("across".into()),
            path: path("a.rs"),
            message: UnknownText(format!("partial {tag}").into()),
            cause: ryo_values::domain::runs::types::UnknownCause::Undecided,
        }, StoredUnknown {
            rule: RuleId("across".into()),
            path: path("a.rs"),
            message: UnknownText(format!("refused {tag}").into()),
            cause: ryo_values::domain::runs::types::UnknownCause::Refused(
                ryo_values::domain::runs::types::ApplicabilityRefusal::SyntaxUndetermined,
            ),
        }],
        skipped: Vec::new(),
        frontiers: vec![StoredFrontier {
            file: StoredFileNumber(0),
            rule: PlacedRule(1),
            digest: Digest([tag; 16]),
        }],
        consulted: vec![StoredFile {
            executable: Default::default(),
            path: FilePath("../.cargo/config.toml".into()),
            digest: Digest([tag.wrapping_add(2); 16]),
            interface: Digest([0; 16]),
            noted: Digest([0; 16]),
            size: FileSize(u64::from(tag)),
            modified: Nanoseconds(9),
        }],
        consulted_absent: vec![FilePath("../.cargo/config".into())],
        project_observations: Vec::new(),
        handed: handed(tag),
    }
}

/// What the files of `stored(tag)` handed.
fn handed(tag: u8) -> StoredHanded {
    StoredHanded(ryo_values::domain::stores::types::HandedRows {
        observations: Vec::new(),
        proofs: vec![ryo_values::domain::stores::types::StoredFrontierProof {
            file: StoredFileNumber(0),
            bucket: Bucket(2),
            proof: super::proof(),
        }],
        frontier: vec![StoredWords {
            file: StoredFileNumber(0),
            bucket: Bucket(2),
            proof: super::proof(),
            words: PackedWords(vec![1, tag, 0x81, 0x01]),
        }],
        texts: vec![StoredText {
            file: StoredFileNumber(0),
            text: ArenaBytes(b"derived".to_vec()),
        }],
        notes: vec![StoredNotes {
            file: StoredFileNumber(1),
            notes: FileNotes {
                language: LanguageNumber(usize::from(tag)),
                notes: NoteBytes(vec![tag, 0, 255]),
            },
        }],
        covered: vec![StoredCovered {
            file: StoredFileNumber(0),
            governing: GoverningRules(vec![0, 1]),
            partials: vec![StoredPartial {
                relation_name: RelationName("function".into()),
                cause: PartialCause("syntax error".into()),
                lacking_kind: PartialKind(Box::default()),
                at: PartialScope::Repository,
            }],
        }],
    })
}

/// The same as `stored`, with a file no language could parse.
#[must_use]
pub fn failed(tag: u8) -> Stored {
    let mut failed = stored(tag);
    if let Some(each) = failed.unread.first_mut() {
        each.reason = Unreadable::Failed(ReadFailure("it failed".into()));
    }
    failed
}
