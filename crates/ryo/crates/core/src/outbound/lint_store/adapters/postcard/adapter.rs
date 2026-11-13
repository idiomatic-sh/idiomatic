// ignore: idiomatic.arch/file-length - one single-port adapter: the adapter spec places one adapter.rs, and the store file reading and keeping is one sequence over one open snapshot that no placed role splits
//! The store port, implemented with one file in the cache directory: sections one after the other and a table of contents last, so
//! a run reads only the sections it needs; written whole through a temporary file renamed into place, so a reader sees the old store
//! or the new, never part of one. A bucket whose words are as they were is copied from the old file as it is.
use super::connection::invalid;
use super::constants::FILE;
use super::constants::FRESH_ROW_COUNT_EXCEEDS_STAGING_WIDTH;
use super::constants::FRESH_ROW_COUNT_OVERFLOWS;
use super::constants::INVALID_FRESH_STAGED_ROW_ORDER;
use super::constants::LENGTH;
use super::constants::MISSING_INLINE_FRESH_ROW;
use super::constants::MISSING_STAGED_ROW;
use super::constants::NOT_A_FILE;
use super::constants::OUTSIDE;
use super::constants::TEMPORARY;
use super::constants::TRAILER;
use super::constants::UNDECODED;
use super::rest::schemas::rest_bytes;
use super::rest::schemas::rest_from;
use super::schemas::contents;
use super::schemas::contents_bytes;
use super::tables::schemas::tables_bytes;
use super::tables::schemas::tables_of;
use super::types::BucketSpan;
use super::types::FileOffset;
use super::types::Frame;
use super::types::OpenedStore;
use super::types::Payload;
use super::types::SectionBytes;
use super::types::SectionLength;
use super::types::SectionSpan;
use super::types::Sequence;
use super::types::TableOfContents;
use super::types::WriterNumber;
use crate::outbound::lint_store::adapters::postcard::constants::FRONTIER_PAYLOAD_LACKS_ITS_EXACT_SOURCE;
use crate::outbound::lint_store::adapters::postcard::constants::FRONTIER_PROOFS_HAVE_DUPLICATE_OR_UNORDERED;
use crate::outbound::lint_store::adapters::postcard::constants::NO_STAGING_SESSION_WAS_READ;
#[cfg(unix)]
use crate::outbound::lint_store::adapters::postcard::constants::OLD_BUCKET_END_OVERFLOWS;
#[cfg(unix)]
use crate::outbound::lint_store::adapters::postcard::constants::OLD_BUCKET_LOGICAL_POSITION_OVERFLOWS;
#[cfg(unix)]
use crate::outbound::lint_store::adapters::postcard::constants::OLD_BUCKET_POSITION_OVERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_BUCKET_SEEK_EXCEEDS_ITS_SECTION;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_BELONGS_TO_ANOTHER_STORE;
#[cfg(unix)]
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_BUCKET_IS_ABSENT;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_BUCKET_OR_SPAN_IS;
use crate::outbound::lint_store::adapters::postcard::constants::STORE_SNAPSHOT_IDENTITIES_EXHAUSTED;
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::interface::LintStorePort;
use crate::outbound::lint_store::interface::OldRowBatch;
use crate::outbound::lint_store::interface::ReadStore;
use crate::outbound::lint_store::types::CacheDirectory;
use crate::outbound::lint_store::types::Store;
use crate::outbound::lint_store::types::StoreKeeping;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::BucketsWanted;
use ryo_values::domain::stores::types::KeptRows;
use ryo_values::domain::stores::types::OldWords;
use ryo_values::domain::stores::types::ReadHanded;
use ryo_values::domain::stores::types::ReadWordPayload;
use ryo_values::domain::stores::types::ReadWords;
use ryo_values::domain::stores::types::StagedRow;
use ryo_values::domain::stores::types::StoreSnapshot;
use ryo_values::domain::stores::types::Stored;
use ryo_values::domain::stores::types::StoredWords;
use std::collections::BTreeMap;
use std::io::Read as _;
use std::io::Seek as _;
use std::io::SeekFrom;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;
use std::sync::atomic::Ordering;

/// The postcard store adapter: the store of one cache directory, for one build.
#[derive(Clone, Debug)]
pub(crate) struct LintStorePostcardAdapter {
    /// Where the store is kept, and the build that keeps it.
    store: Store,
    /// The count that gives each reader and each writer of this adapter a number of its own.
    sequence: Sequence,
}

/// The store as one run read it.
#[derive(Debug)]
struct PostcardRead {
    /// The adapter of the store's place and build.
    adapter: LintStorePostcardAdapter,
    /// The store file as found, open; none when there was none.
    opened: Option<Mutex<OpenedStore>>,
    /// Where the small tables are in the file.
    tables: SectionSpan,
    /// Immutable bucket framing metadata retained when this reader opens its snapshot.
    buckets: Vec<BucketSpan>,
    /// One snapshot identity, even before it creates a temporary file.
    session: ryo_values::domain::records::types::StagingSession,
    /// Fresh normalized data is staged lazily, outside domain record ownership.
    spool: Mutex<Option<super::connection::StagingConnection>>,
}

#[cfg(unix)]
struct PositionalBucket {
    file: Arc<std::fs::File>,
    start: FileOffset,
    end: FileOffset,
    at: FileOffset,
}

#[cfg(unix)]
impl std::io::Read for PositionalBucket {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        use std::os::unix::fs::FileExt as _;
        let length = into
            .len()
            .min(usize::try_from(self.end.0 - self.at.0).unwrap_or(usize::MAX));
        let read = self.file.read_at(&mut into[..length], self.at.0)?;
        self.at = FileOffset(
            self.at
                .0
                .checked_add(u64::try_from(read).map_err(std::io::Error::other)?)
                .ok_or_else(|| std::io::Error::other(OLD_BUCKET_POSITION_OVERFLOWS))?,
        );
        Ok(read)
    }
}

#[cfg(unix)]
impl std::io::Seek for PositionalBucket {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let target = match from {
            SeekFrom::Start(at) => i128::from(at),
            SeekFrom::Current(by) => i128::from(self.at.0) + i128::from(by),
            SeekFrom::End(by) => i128::from(self.end.0) + i128::from(by),
        };
        let at = u64::try_from(target).map_err(std::io::Error::other)?;
        if at < self.start.0 || at > self.end.0 {
            return Err(std::io::Error::other(OLD_BUCKET_SEEK_EXCEEDS_ITS_SECTION));
        }
        self.at = FileOffset(at);
        Ok(at)
    }
}

/// A standard 4 KiB read buffer whose logical seek can reuse prefetched bytes.
#[cfg(unix)]
struct BufferedBucket {
    bucket: Bucket,
    span: SectionSpan,
    reader: std::io::BufReader<PositionalBucket>,
    at: FileOffset,
}

#[cfg(unix)]
impl std::io::Read for BufferedBucket {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        let read = self.reader.read(into)?;
        self.at = FileOffset(
            self.at
                .0
                .checked_add(u64::try_from(read).map_err(std::io::Error::other)?)
                .ok_or_else(|| std::io::Error::other(OLD_BUCKET_LOGICAL_POSITION_OVERFLOWS))?,
        );
        Ok(read)
    }
}

#[cfg(unix)]
impl std::io::Seek for BufferedBucket {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let end = self
            .span
            .start
            .0
            .checked_add(self.span.length.0)
            .ok_or_else(|| std::io::Error::other(OLD_BUCKET_END_OVERFLOWS))?;
        let target = match from {
            SeekFrom::Start(at) => i128::from(at),
            SeekFrom::Current(by) => i128::from(self.at.0) + i128::from(by),
            SeekFrom::End(by) => i128::from(end) + i128::from(by),
        };
        let at = u64::try_from(target).map_err(std::io::Error::other)?;
        if at < self.span.start.0 || at > end {
            return Err(std::io::Error::other(OLD_BUCKET_SEEK_EXCEEDS_ITS_SECTION));
        }
        let delta = i128::from(at) - i128::from(self.at.0);
        if let Ok(delta) = i64::try_from(delta) {
            self.reader.seek_relative(delta)?;
        } else {
            let _position = self.reader.seek(SeekFrom::Start(at))?;
        }
        self.at = FileOffset(at);
        Ok(at)
    }
}

#[cfg(unix)]
struct PostcardOldRowBatch {
    snapshot: StoreSnapshot,
    held: SectionLength,
    buckets: Vec<BufferedBucket>,
    directory: CacheDirectory,
}

#[cfg(unix)]
impl OldRowBatch for PostcardOldRowBatch {
    fn read(
        &mut self,
        words: &OldWords,
        into: &mut ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<(), LintStoreError> {
        into.0.clear();
        let place = self
            .buckets
            .binary_search_by_key(&words.bucket, |row| row.bucket)
            .map_err(|_| LintStoreError::Unreadable {
                directory: self.directory.0.to_string(),
                reason: OLD_ROW_BUCKET_IS_ABSENT.into(),
            })?;
        let bucket = &mut self.buckets[place];
        let span = bucket.span;
        super::writer::read_old_in(bucket, self.held, self.snapshot, span, words, into).map_err(
            |error| LintStoreError::Unreadable {
                directory: self.directory.0.to_string(),
                reason: error.to_string(),
            },
        )
    }
}

impl LintStorePostcardAdapter {
    /// An adapter over the store `store` names.
    pub(crate) fn new(store: Store) -> Self {
        Self {
            store,
            sequence: Sequence::default(),
        }
    }

    /// Never wrap a live snapshot identity or reuse a writer name: the adapter counts its readers and its writers with one checked
    /// sequence of its own.
    fn unique(&self) -> std::io::Result<WriterNumber> {
        self.sequence
            .0
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map(WriterNumber)
            .map_err(|_| std::io::Error::other(STORE_SNAPSHOT_IDENTITIES_EXHAUSTED))
    }

    /// What the files of the open store `opened` handed: what they handed but their words, and the words of the buckets `wanted`
    /// alone; empty when the store is another build's.
    fn handed_from(
        &self,
        opened: &mut OpenedStore,
        wanted: &BucketsWanted,
        snapshot: StoreSnapshot,
    ) -> Result<ReadHanded, LintStoreError> {
        let Some(contents) = contents(&self.store.build.0, &opened.contents.0) else {
            return Ok(ReadHanded::default());
        };
        let file = &mut opened.file;
        let rest = section(file, contents.rest).map_err(|error| self.unreadable(&error))?;
        let mut handed = ryo_values::domain::stores::service::read_handed(
            rest_from(&rest.0).ok_or_else(|| self.unreadable(&UNDECODED))?,
        );
        for each in &contents.buckets {
            if let BucketsWanted::These(buckets) = wanted
                && buckets.binary_search(&each.bucket).is_err()
            {
                continue;
            }
            for row in super::writer::old_word_rows(file, each.span)
                .map_err(|error| self.unreadable(&error))?
            {
                let Ok(at) = handed
                    .proofs
                    .binary_search_by_key(&(row.file.0, each.bucket.0), |entry| {
                        (entry.file.0, entry.bucket.0)
                    })
                else {
                    return Err(self.unreadable(&UNDECODED));
                };
                let proof = handed
                    .proofs
                    .get(at)
                    .ok_or_else(|| self.unreadable(&UNDECODED))?
                    .proof
                    .clone();
                handed.frontier.push(ReadWords {
                    file: row.file,
                    bucket: each.bucket,
                    proof,
                    payload: ReadWordPayload::Old(OldWords {
                        snapshot,
                        bucket: each.bucket,
                        file: row.file,
                        header: row.header,
                        offset: ryo_values::domain::records::types::PayloadOffset(row.span.start.0),
                        length: ryo_values::domain::records::types::PayloadLength(
                            row.span.length.0,
                        ),
                    }),
                });
            }
        }
        handed.frontier.sort_by_key(|each| (each.file, each.bucket));
        Ok(handed)
    }

    /// The file of the store.
    fn file(&self) -> PathBuf {
        Path::new(&*self.store.directory.0).join(FILE)
    }

    /// The failure to read the store, saying why.
    fn unreadable(&self, reason: &dyn std::fmt::Display) -> LintStoreError {
        LintStoreError::Unreadable {
            directory: self.store.directory.0.to_string(),
            reason: reason.to_string(),
        }
    }

    /// The store file, open, and its table of contents as bytes; none when there is no file or it is too short to hold one.
    fn opened(&self) -> Result<Option<OpenedStore>, LintStoreError> {
        let mut file = match std::fs::File::open(self.file()) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(self.unreadable(&error)),
        };
        let metadata = file.metadata().map_err(|error| self.unreadable(&error))?;
        if !metadata.is_file() {
            return Err(self.unreadable(&NOT_A_FILE));
        }
        let size = metadata.len();
        let trailer_length =
            LENGTH + u64::try_from(TRAILER.len()).map_err(|error| self.unreadable(&error))?;
        if size < trailer_length {
            return Ok(None);
        }
        let mut trailer = SectionBytes(vec![0_u8; 8 + TRAILER.len()]);
        read_at(&mut file, FileOffset(size - trailer_length), &mut trailer)
            .map_err(|error| self.unreadable(&error))?;
        if trailer.0.get(8..) != Some(TRAILER) {
            return Ok(None);
        }
        let length = u64::from_le_bytes(std::array::from_fn(|place| {
            trailer.0.get(place).copied().unwrap_or_default()
        }));
        let Some(start) = (size - trailer_length).checked_sub(length) else {
            return Ok(None);
        };
        let mut bytes = SectionBytes(vec![0_u8; usize::try_from(length).unwrap_or_default()]);
        read_at(&mut file, FileOffset(start), &mut bytes)
            .map_err(|error| self.unreadable(&error))?;
        Ok(Some(OpenedStore {
            file,
            length: SectionLength(size),
            contents: bytes,
        }))
    }
}

/// Nothing when the section `span` ends inside `file`; an error when it reaches past its end.
fn inside(file: &std::fs::File, span: SectionSpan) -> std::io::Result<()> {
    let held = file.metadata()?.len();
    let end = span.start.0.checked_add(span.length.0);
    end.filter(|end| *end <= held)
        .map(|_| ())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, OUTSIDE))
}

/// Reads as many bytes of `file` as `into` holds, from `start`.
fn read_at(
    file: &mut std::fs::File,
    start: FileOffset,
    into: &mut SectionBytes,
) -> std::io::Result<()> {
    let _at = file.seek(SeekFrom::Start(start.0))?;
    file.read_exact(&mut into.0)
}

/// The bytes of the section `span` names; an error when it reaches past the end of the file, before any byte is allocated for it.
fn section(file: &mut std::fs::File, span: SectionSpan) -> std::io::Result<SectionBytes> {
    inside(file, span)?;
    let mut bytes = SectionBytes(vec![
        0_u8;
        usize::try_from(span.length.0).unwrap_or_default()
    ]);
    read_at(file, span.start, &mut bytes)?;
    Ok(bytes)
}

/// Writes `bytes` at `at`, the end of what `out` holds, as a section: answers where it is, and moves `at` past it.
fn put(
    out: &mut std::io::BufWriter<std::fs::File>,
    at: &mut FileOffset,
    bytes: &SectionBytes,
) -> std::io::Result<SectionSpan> {
    out.write_all(&bytes.0)?;
    let span = SectionSpan {
        start: *at,
        length: SectionLength(u64::try_from(bytes.0.len()).unwrap_or(u64::MAX)),
    };
    *at = FileOffset(at.0 + span.length.0);
    Ok(span)
}

/// Copies the section `span` of the store `from` to `at`, the end of what `out` holds, as it is: answers where it is now, and moves
/// `at` past it; an error when it reaches past the end of `from`.
fn copied(
    out: &mut std::io::BufWriter<std::fs::File>,
    at: &mut FileOffset,
    from: &mut std::fs::File,
    span: SectionSpan,
) -> std::io::Result<SectionSpan> {
    inside(from, span)?;
    let _at = from.seek(SeekFrom::Start(span.start.0))?;
    let length = std::io::copy(&mut from.take(span.length.0), out)?;
    if length != span.length.0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            OUTSIDE,
        ));
    }
    let copied = SectionSpan {
        start: *at,
        length: SectionLength(length),
    };
    *at = FileOffset(at.0 + length);
    Ok(copied)
}

/// Writes the buckets the run made again at `at`, the end of what `out` holds: each holds the words of the files read now,
/// `by_bucket`, and, copied from its section in the store `old` as it was (`old_contents`), those of the files used again, under their
/// numbers now. Answers where each is; a bucket no file hands anything is no section.
fn put_made(
    out: &mut std::io::BufWriter<std::fs::File>,
    at: &mut FileOffset,
    mut by_bucket: BTreeMap<Bucket, Vec<super::types::Frame<'_>>>,
    mut old: Option<&mut OpenedStore>,
    old_contents: Option<&[BucketSpan]>,
    kept: &KeptRows,
    mut spool: Option<&mut super::connection::StagingConnection>,
) -> std::io::Result<Vec<BucketSpan>> {
    let mut made: BTreeMap<Bucket, Option<SectionSpan>> =
        by_bucket.keys().map(|bucket| (*bucket, None)).collect();
    if !kept.file_again.is_empty() {
        for each in old_contents.unwrap_or_default() {
            if kept.unchanged.binary_search(&each.bucket).is_err() {
                let _ = made.insert(each.bucket, Some(each.span));
            }
        }
    }
    let mut buckets = Vec::new();
    for (bucket, was) in made {
        let mut rows = by_bucket.remove(&bucket).unwrap_or_default();
        if let (Some(span), Some(opened)) = (was, old.as_deref_mut()) {
            for row in super::writer::old_rows(&mut opened.file, span)? {
                if let Ok(place) = kept
                    .file_again
                    .binary_search_by_key(&row.file.0, |again| again.was.0)
                    && let Some(again) = kept.file_again.get(place)
                {
                    rows.push(super::types::Frame {
                        file: again.now,
                        payload: row.payload,
                    });
                }
            }
            // Stable sorting matches rebuilt_bucket_bytes: fresh ties precede old ties.
            rows.sort_by_key(|row| row.file);
        }
        if rows.is_empty() {
            continue;
        }
        let span = super::writer::write_bucket(
            out,
            at,
            &rows,
            old.as_deref_mut().map(|opened| &mut opened.file),
            spool.as_deref_mut(),
        )?;
        buckets.push(BucketSpan { bucket, span });
    }
    Ok(buckets)
}

/// The open store of a run, locked for as long as the guard lives.
fn locked(opened: &Mutex<OpenedStore>) -> MutexGuard<'_, OpenedStore> {
    opened.lock().unwrap_or_else(PoisonError::into_inner)
}

impl LintStorePort for LintStorePostcardAdapter {
    fn keeps(&self) -> StoreKeeping {
        StoreKeeping::Keeps
    }

    fn stored(&self) -> Result<Arc<dyn ReadStore>, LintStoreError> {
        let session = ryo_values::domain::records::types::StagingSession(
            self.unique().map_err(|error| self.unreadable(&error))?.0,
        );
        let nothing = || {
            Arc::new(PostcardRead {
                adapter: self.clone(),
                opened: None,
                tables: SectionSpan::default(),
                buckets: Vec::new(),
                session,
                spool: Mutex::new(None),
            })
        };
        let Some(opened) = self.opened()? else {
            return Ok(nothing());
        };
        // A section that does not end inside the file is no store: the tables read as empty, and every file is read again.
        let tables = contents(&self.store.build.0, &opened.contents.0).filter(|contents| {
            std::iter::once(contents.tables)
                .chain(std::iter::once(contents.rest))
                .chain(contents.buckets.iter().map(|each| each.span))
                .all(|span| inside(&opened.file, span).is_ok())
        });
        let Some(contents) = tables else {
            return Ok(nothing());
        };
        Ok(Arc::new(PostcardRead {
            adapter: self.clone(),
            opened: Some(Mutex::new(opened)),
            tables: contents.tables,
            buckets: contents.buckets,
            session,
            spool: Mutex::new(None),
        }))
    }
}

impl ReadStore for PostcardRead {
    fn tables(&self) -> Stored {
        // Decoded when asked, so the run holds one copy of the tables: its own.
        self.opened.as_ref().map_or_else(Stored::default, |opened| {
            section(&mut locked(opened).file, self.tables)
                .map_or_else(|_| Stored::default(), |bytes| tables_of(&bytes.0))
        })
    }

    fn handed(&self, wanted: &BucketsWanted) -> Result<ReadHanded, LintStoreError> {
        match &self.opened {
            Some(opened) => {
                self.adapter
                    .handed_from(&mut locked(opened), wanted, StoreSnapshot(self.session.0))
            }
            None => Ok(ReadHanded::default()),
        }
    }

    fn read_old(
        &self,
        words: &OldWords,
        into: &mut ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<(), LintStoreError> {
        if words.snapshot != StoreSnapshot(self.session.0) {
            return Err(self.adapter.unreadable(&OLD_ROW_BELONGS_TO_ANOTHER_STORE));
        }
        let opened = self.opened.as_ref().ok_or(LintStoreError::NothingRead)?;
        #[cfg(unix)]
        let opened = locked(opened);
        #[cfg(not(unix))]
        let mut opened = locked(opened);
        let bucket = self
            .buckets
            .iter()
            .find(|row| {
                row.bucket == words.bucket
                    && row.span.start.0 < words.header.0
                    && row
                        .span
                        .start
                        .0
                        .checked_add(row.span.length.0)
                        .is_some_and(|end| {
                            words
                                .offset
                                .0
                                .checked_add(words.length.0)
                                .is_some_and(|after| after <= end)
                        })
            })
            .ok_or_else(|| self.adapter.unreadable(&OLD_ROW_BUCKET_OR_SPAN_IS))?;
        let held = opened.length;
        #[cfg(unix)]
        let read = super::writer::read_old_file_in(
            &opened.file,
            held,
            StoreSnapshot(self.session.0),
            bucket.span,
            words,
            into,
        );
        #[cfg(not(unix))]
        let read = super::writer::read_old_in(
            &mut opened.file,
            held,
            StoreSnapshot(self.session.0),
            bucket.span,
            words,
            into,
        );
        read.map_err(|error| self.adapter.unreadable(&error))
    }

    fn begin_old_rows(
        &self,
        wanted: &BucketsWanted,
    ) -> Result<Option<Box<dyn OldRowBatch>>, LintStoreError> {
        #[cfg(unix)]
        {
            let opened = self.opened.as_ref().ok_or(LintStoreError::NothingRead)?;
            let opened = locked(opened);
            let file = Arc::new(
                opened
                    .file
                    .try_clone()
                    .map_err(|error| self.adapter.unreadable(&error))?,
            );
            let mut buckets = Vec::with_capacity(self.buckets.len());
            for entry in &self.buckets {
                if let BucketsWanted::These(selected) = wanted
                    && selected.binary_search(&entry.bucket).is_err()
                {
                    continue;
                }
                let end = entry
                    .span
                    .start
                    .0
                    .checked_add(entry.span.length.0)
                    .ok_or_else(|| self.adapter.unreadable(&OLD_BUCKET_END_OVERFLOWS))?;
                let reader = PositionalBucket {
                    file: Arc::clone(&file),
                    start: entry.span.start,
                    end: FileOffset(end),
                    at: entry.span.start,
                };
                buckets.push(BufferedBucket {
                    bucket: entry.bucket,
                    span: entry.span,
                    reader: std::io::BufReader::with_capacity(4 * 1024, reader),
                    at: entry.span.start,
                });
            }
            buckets.sort_by_key(|entry| entry.bucket);
            Ok(Some(Box::new(PostcardOldRowBatch {
                snapshot: StoreSnapshot(self.session.0),
                held: opened.length,
                buckets,
                directory: self.adapter.store.directory.clone(),
            })))
        }
        #[cfg(not(unix))]
        {
            Ok(None)
        }
    }

    fn stage_words(
        &self,
        words: ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<ryo_values::domain::records::types::RecordWords, LintStoreError> {
        let failed = |error: std::io::Error| LintStoreError::Unwritable {
            directory: self.adapter.store.directory.0.to_string(),
            reason: error.to_string(),
        };
        let mut spool = self.spool.lock().unwrap_or_else(PoisonError::into_inner);
        if spool.is_none() {
            *spool = Some(super::connection::StagingConnection::new(self.session).map_err(failed)?);
        }
        let Some(spool) = spool.as_mut() else {
            return Err(LintStoreError::NothingRead);
        };
        spool
            .append(words)
            .map(ryo_values::domain::records::types::RecordWords::Staged)
            .map_err(failed)
    }

    fn read_staged(
        &self,
        words: &ryo_values::domain::records::types::StagedWords,
        into: &mut ryo_values::domain::stored_facts::types::PackedWords,
    ) -> Result<(), LintStoreError> {
        let mut spool = self.spool.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(spool) = spool.as_mut() else {
            return Err(LintStoreError::NothingRead);
        };
        spool
            .read(words, into)
            .map_err(|error| self.adapter.unreadable(&error))
    }

    fn keep(&self, mut stored: Stored, kept: &KeptRows) -> Result<(), LintStoreError> {
        let store = &self.adapter.store;
        let unchanged = &kept.unchanged;
        let unwritable = |reason: &dyn std::fmt::Display| LintStoreError::Unwritable {
            directory: store.directory.0.to_string(),
            reason: reason.to_string(),
        };
        // The buckets as they were, and the words the files used again handed the others, are copied from the store as it is,
        // before it is replaced; a store that cannot be read then cannot be kept whole.
        let read = self.opened.as_ref();
        if read.is_none() && !(unchanged.is_empty() && kept.file_again.is_empty()) {
            return Err(LintStoreError::NothingRead);
        }
        let mut old = read.map(locked);
        let mut spool = self.spool.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(spool) = spool.as_mut() {
            spool.flush().map_err(|error| unwritable(&error))?;
        }
        for row in &kept.staged {
            let held = spool
                .as_ref()
                .ok_or_else(|| unwritable(&NO_STAGING_SESSION_WAS_READ))?;
            super::connection::validate(held.session, held.length, &row.words)
                .map_err(|error| unwritable(&error))?;
        }
        let old_tables = old
            .as_ref()
            .and_then(|opened| contents(&store.build.0, &opened.contents.0));
        if let (Some(opened), Some(tables)) = (old.as_deref_mut(), old_tables.as_ref()) {
            let rest = section(&mut opened.file, tables.rest)
                .map_err(|error| self.adapter.unreadable(&error))?;
            let previous = rest_from(&rest.0).ok_or_else(|| self.adapter.unreadable(&UNDECODED))?;
            ryo_values::domain::stores::service::carry_proofs(&mut stored.handed, &previous, kept);
        }
        stored
            .handed
            .proofs
            .sort_by_key(|row| (row.file.0, row.bucket.0));
        let proofs = &stored.handed.proofs;
        if proofs
            .windows(2)
            .any(|pair| (pair[0].file.0, pair[0].bucket.0) >= (pair[1].file.0, pair[1].bucket.0))
        {
            return Err(unwritable(&FRONTIER_PROOFS_HAVE_DUPLICATE_OR_UNORDERED));
        }
        let proof_for = |file: ryo_values::domain::stored_findings::types::StoredFileNumber,
                         bucket: Bucket| {
            let Ok(at) =
                proofs.binary_search_by_key(&(file.0, bucket.0), |row| (row.file.0, row.bucket.0))
            else {
                return None;
            };
            proofs.get(at).map(|row| &row.proof)
        };
        if stored
            .handed
            .frontier
            .iter()
            .any(|row| proof_for(row.file, row.bucket) != Some(&row.proof))
            || kept
                .staged
                .iter()
                .any(|row| proof_for(row.file, row.bucket).is_none())
        {
            return Err(unwritable(&FRONTIER_PAYLOAD_LACKS_ITS_EXACT_SOURCE));
        }
        let by_bucket =
            fresh(&stored.handed.frontier, &kept.staged).map_err(|error| unwritable(&error))?;
        let old_contents = old_tables.map(|each| each.buckets);
        std::fs::create_dir_all(Path::new(&*store.directory.0))
            .map_err(|error| unwritable(&error))?;
        let temporary = Path::new(&*store.directory.0).join(format!(
            "{TEMPORARY}.{}.{:x}.{}",
            std::process::id(),
            Arc::as_ptr(&self.adapter.sequence.0).addr(),
            self.adapter.unique().map_err(|error| unwritable(&error))?.0
        ));
        let written = std::fs::File::create(&temporary)
            .map_err(|error| unwritable(&error))
            .and_then(|file| {
                let mut out =
                    std::io::BufWriter::with_capacity(super::connection::BUFFER_BYTES, file);
                let mut at = FileOffset(0);
                let tables = put(
                    &mut out,
                    &mut at,
                    &SectionBytes(tables_bytes(&stored).map_err(|error| unwritable(&error))?),
                )
                .map_err(|error| unwritable(&error))?;
                let rest = put(
                    &mut out,
                    &mut at,
                    &SectionBytes(rest_bytes(&stored.handed).map_err(|error| unwritable(&error))?),
                )
                .map_err(|error| unwritable(&error))?;
                let mut buckets = put_made(
                    &mut out,
                    &mut at,
                    by_bucket,
                    old.as_deref_mut(),
                    old_contents.as_deref(),
                    kept,
                    spool.as_mut(),
                )
                .map_err(|error| unwritable(&error))?;
                // The buckets as they were are copied as they are, byte for byte: none is decoded or encoded again.
                if let (Some(was), Some(spans)) = (old.as_deref_mut(), &old_contents) {
                    for each in spans {
                        if buckets.iter().any(|made| made.bucket == each.bucket)
                            || unchanged.binary_search(&each.bucket).is_err()
                        {
                            continue;
                        }
                        buckets.push(BucketSpan {
                            bucket: each.bucket,
                            span: copied(&mut out, &mut at, &mut was.file, each.span)
                                .map_err(|error| unwritable(&error))?,
                        });
                    }
                }
                buckets.sort_by_key(|each| each.bucket);
                let toc = SectionBytes(
                    contents_bytes(
                        &store.build.0,
                        &TableOfContents {
                            tables,
                            rest,
                            buckets,
                        },
                    )
                    .map_err(|error| unwritable(&error))?,
                );
                let length = u64::try_from(toc.0.len()).unwrap_or(u64::MAX);
                let _contents = put(&mut out, &mut at, &toc).map_err(|error| unwritable(&error))?;
                out.write_all(&length.to_le_bytes())
                    .map_err(|error| unwritable(&error))?;
                out.write_all(TRAILER).map_err(|error| unwritable(&error))?;
                out.flush().map_err(|error| unwritable(&error))
            });
        published(&temporary, written, || {
            std::fs::rename(&temporary, self.adapter.file()).map_err(|error| unwritable(&error))
        })
    }
}

/// Publication follows a fully flushed write. Failure never removes the previous cache target.
pub(super) fn published<E>(
    temporary: &Path,
    written: Result<(), E>,
    rename: impl FnOnce() -> Result<(), E>,
) -> Result<(), E> {
    let result = written.and_then(|()| rename());
    if result.is_err() {
        // Preserve the original failure; an uncommitted temporary file is never read as a store.
        drop(std::fs::remove_file(temporary));
    }
    result
}

/// Restore one fresh sequence from inline entries and staged ordinal slots; no payload is copied.
pub(super) fn fresh<'a>(
    inline: &'a [StoredWords],
    staged: &[StagedRow],
) -> std::io::Result<BTreeMap<Bucket, Vec<Frame<'a>>>> {
    let count = inline
        .len()
        .checked_add(staged.len())
        .ok_or_else(|| invalid(FRESH_ROW_COUNT_OVERFLOWS))?;
    let mut spans: Vec<_> = staged.iter().collect();
    spans.sort_by_key(|row| row.ordinal);
    if spans
        .iter()
        .any(|row| !usize::try_from(row.ordinal.0).is_ok_and(|ordinal| ordinal < count))
        || spans
            .windows(2)
            .any(|pair| pair[0].ordinal == pair[1].ordinal)
    {
        return Err(invalid(INVALID_FRESH_STAGED_ROW_ORDER));
    }
    let mut spans = spans.into_iter().peekable();
    let mut inline = inline.iter();
    let mut buckets: BTreeMap<Bucket, Vec<Frame<'a>>> = BTreeMap::new();
    for ordinal in
        0..u64::try_from(count).map_err(|_| invalid(FRESH_ROW_COUNT_EXCEEDS_STAGING_WIDTH))?
    {
        if spans.peek().is_some_and(|row| row.ordinal.0 == ordinal) {
            let row = spans.next().ok_or_else(|| invalid(MISSING_STAGED_ROW))?;
            buckets.entry(row.bucket).or_default().push(Frame {
                file: row.file,
                payload: Payload::Staged(row.words),
            });
        } else {
            let row = inline
                .next()
                .ok_or_else(|| invalid(MISSING_INLINE_FRESH_ROW))?;
            buckets.entry(row.bucket).or_default().push(Frame {
                file: row.file,
                payload: Payload::Inline(&row.words),
            });
        }
    }
    Ok(buckets)
}
