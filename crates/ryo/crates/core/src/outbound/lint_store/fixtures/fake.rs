//! The fake of the `lint_store` port: what the core's tests stand in for it with, and the tables they keep in it.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use super::super::errors::LintStoreError;
use super::super::interface::{LintStorePort, ReadStore};
use super::super::types::StoreKeeping;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::reasons::types::ToolMessage;
use ryo_language::domain::relations::types::{Offset, Span};
use ryo_values::domain::descriptions::types::{LanguageName, RelationName};
use ryo_values::domain::files::types::{FileSize, Nanoseconds};
use ryo_values::domain::program::types::ColumnName;
use ryo_values::domain::records::types::Cell;
use ryo_values::domain::records::types::{
    GoverningRules, PlacedColumn, PlacedLine, PlacedRule, Unreadable,
};
use ryo_values::domain::runs::types::{ManifestRefused, RuleId};
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_facts::types::{
    ArenaBytes, Bucket, Digest, FileNotes, LanguageNumber, NoteBytes, PartialCause, PartialKind,
    PartialScope, ReadFailure, StoredPartial,
};
use ryo_values::domain::stores::types::{
    BucketsWanted, Crossing, FileAgain, FindingMade, FindingPlace, KeptRows, Stored, StoredCell,
    StoredCovered, StoredFile, StoredFileNumber, StoredFinding, StoredFrontier, StoredHanded,
    StoredNotes, StoredPlace, StoredRule, StoredText, StoredUnknown, StoredUnread, StoredValue,
    StoredWords, UnknownText,
};

/// What the core's tests stand in for `lint_store` with: the tables kept in memory, and how often each call was made.
#[derive(Debug, Default)]
pub struct LintStoreFake {
    /// What the last run kept.
    pub kept: Arc<Mutex<Stored>>,
    /// Whether every read fails.
    pub unreadable: bool,
    /// Whether every keep fails.
    pub unwritable: bool,
    /// How many times the tables were read.
    pub reads: Arc<AtomicUsize>,
    /// How many times the tables were kept.
    pub keeps: Arc<AtomicUsize>,
}

/// The fake's store as one run read it: what it held then, and the places the next keep and the counts go to.
#[derive(Debug)]
pub struct FakeRead {
    /// The fake the run read.
    fake: LintStoreFake,
    /// What it held when the run read it.
    was: Stored,
}

impl LintStorePort for LintStoreFake {
    fn keeps(&self) -> StoreKeeping {
        StoreKeeping::Keeps
    }

    fn stored(&self) -> Result<Arc<dyn ReadStore>, LintStoreError> {
        let _reads_before = self.reads.fetch_add(1, Ordering::Relaxed);
        if self.unreadable {
            return Err(LintStoreError::Unreadable {
                directory: String::new(),
                reason: String::new(),
            });
        }
        let was = self
            .kept
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        Ok(Arc::new(FakeRead {
            fake: LintStoreFake {
                kept: Arc::clone(&self.kept),
                unreadable: self.unreadable,
                unwritable: self.unwritable,
                reads: Arc::clone(&self.reads),
                keeps: Arc::clone(&self.keeps),
            },
            was,
        }))
    }
}

impl ReadStore for FakeRead {
    fn tables(&self) -> Stored {
        Stored {
            handed: StoredHanded::default(),
            ..self.was.clone()
        }
    }

    fn handed(&self, wanted: &BucketsWanted) -> Result<StoredHanded, LintStoreError> {
        let _reads_before = self.fake.reads.fetch_add(1, Ordering::Relaxed);
        if self.fake.unreadable {
            return Err(LintStoreError::Unreadable {
                directory: String::new(),
                reason: String::new(),
            });
        }
        let mut handed = self.was.handed.clone();
        if let BucketsWanted::These(buckets) = wanted {
            handed
                .frontier
                .retain(|each| buckets.binary_search(&each.bucket).is_ok());
        }
        Ok(handed)
    }

    fn keep(&self, mut stored: Stored, kept: &KeptRows) -> Result<(), LintStoreError> {
        let _keeps_before = self.fake.keeps.fetch_add(1, Ordering::Relaxed);
        if self.fake.unwritable {
            return Err(LintStoreError::Unwritable {
                directory: String::new(),
                reason: String::new(),
            });
        }
        if self.was == Stored::default()
            && !(kept.unchanged.is_empty() && kept.file_again.is_empty())
        {
            return Err(LintStoreError::NothingRead);
        }
        // The words of the buckets as they were stay as they were kept, unless the run made them again; in every other bucket the
        // words of a file used again stay, under its number now.
        let made: Vec<Bucket> = stored
            .handed
            .frontier
            .iter()
            .map(|each| each.bucket)
            .collect();
        stored
            .handed
            .frontier
            .extend(self.was.handed.frontier.iter().filter_map(|each| {
                if kept.unchanged.binary_search(&each.bucket).is_ok() {
                    return (!made.contains(&each.bucket)).then(|| each.clone());
                }
                // A file not used again is gone, with its words.
                match kept
                    .file_again
                    .binary_search_by_key(&each.file, |again| again.was)
                {
                    Ok(place) => kept.file_again.get(place).map(|again| StoredWords {
                        file: again.now,
                        ..each.clone()
                    }),
                    Err(_) => None,
                }
            }));
        stored
            .handed
            .frontier
            .sort_by_key(|each| (each.file, each.bucket));
        *self
            .fake
            .kept
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = stored;
        Ok(())
    }
}

/// The words one file handed, in bucket 3.
#[must_use]
pub fn words_of(file: u32, word: u8) -> StoredWords {
    StoredWords {
        file: StoredFileNumber(file),
        bucket: Bucket(3),
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
    (
        second,
        KeptRows {
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
                path: path("a.rs"),
                digest: Digest([1; 16]),
                interface: Digest([2; 16]),
                noted: Digest([4; 16]),
                size: FileSize(u64::from(tag)),
                modified: Nanoseconds(7),
            },
            StoredFile {
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
        }],
        frontiers: vec![StoredFrontier {
            file: StoredFileNumber(0),
            rule: PlacedRule(1),
            digest: Digest([tag; 16]),
        }],
        consulted: vec![StoredFile {
            path: FilePath("../.cargo/config.toml".into()),
            digest: Digest([tag.wrapping_add(2); 16]),
            interface: Digest([0; 16]),
            noted: Digest([0; 16]),
            size: FileSize(u64::from(tag)),
            modified: Nanoseconds(9),
        }],
        consulted_absent: vec![FilePath("../.cargo/config".into())],
        handed: handed(tag),
    }
}

/// What the files of `stored(tag)` handed.
fn handed(tag: u8) -> StoredHanded {
    StoredHanded {
        frontier: vec![StoredWords {
            file: StoredFileNumber(0),
            bucket: Bucket(2),
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
    }
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

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
