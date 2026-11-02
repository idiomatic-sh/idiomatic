// ignore: idiomatic.arch/adapter/stateless-adapters - WRITERS is the one counter that gives each writer a file name of its own so two runs never write one file; the name must be unique across threads, which only a process-global can promise. It is the only state: the store a run read travels with the run, in its ReadStore.
//! The store port, implemented with one file in the cache directory: sections one after the other and a table of contents last, so
//! a run reads only the sections it needs; written whole through a temporary file renamed into place, so a reader sees the old store
//! or the new, never part of one. A bucket whose words are as they were is copied from the old file as it is.

use std::collections::BTreeMap;
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::constants::{FILE, LENGTH, NOT_A_FILE, OUTSIDE, TEMPORARY, UNDECODED};
use super::schemas::{contents, contents_bytes, rest_bytes, rest_from, tables_bytes, tables_of};
use super::types::{
    BucketSpan, FileOffset, OpenedStore, SectionBytes, SectionLength, SectionSpan, TableOfContents,
};
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::interface::{LintStorePort, ReadStore};
use crate::outbound::lint_store::types::{Store, StoreKeeping};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{
    BucketsWanted, KeptRows, OldWords, ReadHanded, ReadWordPayload, ReadWords, StoreSnapshot,
    Stored,
};

/// The postcard store adapter: the store of one cache directory, for one build.
#[derive(Clone, Debug)]
pub(crate) struct LintStorePostcardAdapter {
    /// Where the store is kept, and the build that keeps it.
    store: Store,
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
    spool: Mutex<Option<super::types::spool::value::StagingSpool>>,
}

impl LintStorePostcardAdapter {
    /// An adapter over the store `store` names.
    pub(crate) fn new(store: Store) -> Self {
        Self { store }
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
            for row in super::types::frames::value::old_word_rows(file, each.span)
                .map_err(|error| self.unreadable(&error))?
            {
                handed.frontier.push(ReadWords {
                    file: row.file,
                    bucket: each.bucket,
                    words: ReadWordPayload::Old(OldWords {
                        snapshot,
                        bucket: each.bucket,
                        file: row.file,
                        header: row.header,
                        offset: row.span.start.0,
                        length: row.span.length.0,
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
        if size < LENGTH {
            return Ok(None);
        }
        let mut trailer = SectionBytes(vec![0_u8; 8]);
        read_at(&mut file, FileOffset(size - LENGTH), &mut trailer)
            .map_err(|error| self.unreadable(&error))?;
        let length = u64::from_le_bytes(std::array::from_fn(|place| {
            trailer.0.get(place).copied().unwrap_or_default()
        }));
        let Some(start) = (size - LENGTH).checked_sub(length) else {
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
    mut by_bucket: BTreeMap<Bucket, Vec<super::types::frames::value::Frame<'_>>>,
    mut old: Option<&mut OpenedStore>,
    old_contents: Option<&[BucketSpan]>,
    kept: &KeptRows,
    mut spool: Option<&mut super::types::spool::value::StagingSpool>,
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
            for row in super::types::frames::value::old_rows(&mut opened.file, span)? {
                if let Ok(place) = kept
                    .file_again
                    .binary_search_by_key(&row.file.0, |again| again.was.0)
                    && let Some(again) = kept.file_again.get(place)
                {
                    rows.push(super::types::frames::value::Frame {
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
        let span = super::types::frames::value::write_bucket(
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

/// Writer names and staging sessions share one checked process-unique sequence.
static WRITERS: AtomicU64 = AtomicU64::new(0);

/// Never wrap a live snapshot identity or reuse a writer name.
fn unique() -> std::io::Result<u64> {
    WRITERS
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| std::io::Error::other("store snapshot identities exhausted"))
}

impl LintStorePort for LintStorePostcardAdapter {
    fn keeps(&self) -> StoreKeeping {
        StoreKeeping::Keeps
    }

    fn stored(&self) -> Result<Arc<dyn ReadStore>, LintStoreError> {
        let session = ryo_values::domain::records::types::StagingSession(
            unique().map_err(|error| self.unreadable(&error))?,
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
            return Err(self
                .adapter
                .unreadable(&"old row belongs to another store snapshot"));
        }
        let opened = self.opened.as_ref().ok_or(LintStoreError::NothingRead)?;
        let mut opened = locked(opened);
        let bucket = self
            .buckets
            .iter()
            .find(|row| {
                row.bucket == words.bucket
                    && row.span.start.0 < words.header
                    && row
                        .span
                        .start
                        .0
                        .checked_add(row.span.length.0)
                        .is_some_and(|end| {
                            words
                                .offset
                                .checked_add(words.length)
                                .is_some_and(|after| after <= end)
                        })
            })
            .ok_or_else(|| self.adapter.unreadable(&"old row bucket or span is absent"))?;
        let held = opened.length.0;
        super::types::frames::value::read_old_in(
            &mut opened.file,
            held,
            StoreSnapshot(self.session.0),
            bucket.span,
            words,
            into,
        )
        .map_err(|error| self.adapter.unreadable(&error))
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
            *spool =
                Some(super::types::spool::value::StagingSpool::new(self.session).map_err(failed)?);
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

    fn keep(&self, stored: Stored, kept: &KeptRows) -> Result<(), LintStoreError> {
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
                .ok_or_else(|| unwritable(&"no staging session was read"))?;
            super::types::spool::value::validate(held.session, held.length, &row.words)
                .map_err(|error| unwritable(&error))?;
        }
        let by_bucket = super::types::frames::value::fresh(&stored.handed.frontier, &kept.staged)
            .map_err(|error| unwritable(&error))?;
        let old_contents = old
            .as_ref()
            .and_then(|opened| contents(&store.build.0, &opened.contents.0))
            .map(|each| each.buckets);
        std::fs::create_dir_all(Path::new(&*store.directory.0))
            .map_err(|error| unwritable(&error))?;
        let temporary = Path::new(&*store.directory.0).join(format!(
            "{TEMPORARY}.{}.{}",
            std::process::id(),
            unique().map_err(|error| unwritable(&error))?
        ));
        let written = std::fs::File::create(&temporary)
            .map_err(|error| unwritable(&error))
            .and_then(|file| {
                let mut out = std::io::BufWriter::with_capacity(
                    super::types::spool::value::BUFFER_BYTES,
                    file,
                );
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
