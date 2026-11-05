//! The facts port, with the rows of a worker's file in columns of its own and the words in a file: what a worker keeps for the
//! cross-file finish is written as it is kept and read back when the finish asks for a bucket, so it does not wait in memory for it.

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ryo_relations::domain::relations::service::{all, columns, kinds};
use ryo_relations::domain::relations::types::Relation;

use super::constants::{FILE, FLUSH_AT, MISSING};
use super::types::{
    PackedLength, RoomToFill, Spill, SpillFile, SpillOffset, SpillStretch, SpillWorkerNumber,
};
use ryo_execution::inbound::use_cases::execution::types::facts::FactsError;
use ryo_execution::inbound::use_cases::execution::types::facts::{
    BucketSource, ColumnsMut, KeptBucket, WordsToKeep,
};
use ryo_execution::inbound::use_cases::execution::types::facts::{
    FactsFinish, FactsPort, FactsWorker,
};
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
use ryo_values::domain::stored_facts::types::Bucket;

/// The spill facts adapter: the rows of each worker in memory, a file of the adapter's directory for its words.
#[derive(Debug)]
pub(crate) struct FactsSpillAdapter {
    /// The directory the workers' spill files are made in.
    directory: PathBuf,
    /// The number the next worker's handle takes, which names its spill file.
    next: AtomicUsize,
}

impl FactsSpillAdapter {
    /// An adapter that makes its files in `directory`.
    pub(crate) fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            next: AtomicUsize::new(0),
        }
    }

    /// An adapter that makes its files in the system's temporary directory: each is removed from it as soon as it is open, so
    /// nothing is left there.
    pub(crate) fn in_scratch() -> Self {
        Self::new(std::env::temp_dir())
    }
}

/// A failure of the file, as the port says it.
fn spilled(error: &std::io::Error) -> FactsError {
    FactsError::File {
        reason: error.to_string(),
    }
}

/// A refusal of the rows, as the port says it.
fn refused() -> FactsError {
    FactsError::Store {
        reason: MISSING.to_owned(),
    }
}

/// One worker's handle: the columns of each relation for its file, and the file its words go to.
struct SpillWorker {
    /// The worker's number, which names its spill file.
    number: SpillWorkerNumber,
    /// The directory the spill file is made in.
    directory: PathBuf,
    /// The columns of each relation for the worker's file, in the order of all relations.
    tables: Vec<Vec<FactColumn>>,
    /// The file the worker's words go to, and where each bucket's stretches are in it.
    spill: Mutex<Spill>,
}

impl SpillWorker {
    /// A file of the directory, already removed from it: it lasts as long as it is open. Its name is the process's, the
    /// worker's handle and the first attempt no other file of the directory holds: many runs of one process may spill to
    /// one directory at once, and one that finds its name taken takes the next.
    fn open(&self) -> Result<Arc<std::fs::File>, FactsError> {
        let mut attempt = 0_u64;
        loop {
            let path = self.directory.join(format!(
                "{FILE}-{}-{}-{attempt}",
                std::process::id(),
                self.number.0
            ));
            let opened = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path);
            match opened {
                Ok(file) => {
                    std::fs::remove_file(&path).map_err(|error| spilled(&error))?;
                    return Ok(Arc::new(file));
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    attempt = attempt.saturating_add(1);
                }
                Err(error) => return Err(spilled(&error)),
            }
        }
    }
}

/// Writes what waits in memory to the end of the file.
fn flush(spill: &mut Spill) -> Result<(), FactsError> {
    if spill.pending.0.is_empty() {
        return Ok(());
    }
    if let SpillFile::Open(file) = &spill.file {
        let mut writer: &std::fs::File = file;
        writer
            .write_all(&spill.pending.0)
            .map_err(|error| spilled(&error))?;
    }
    spill.written = SpillOffset(
        spill
            .written
            .0
            .saturating_add(u64::try_from(spill.pending.0.len()).unwrap_or(u64::MAX)),
    );
    spill.pending.0.clear();
    Ok(())
}

/// Fills `into` from `file` at `offset`, however the platform reads at a place.
#[cfg(unix)]
fn read_exact_at(
    file: &std::fs::File,
    RoomToFill(into): RoomToFill<'_>,
    offset: SpillOffset,
) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::read_exact_at(file, into, offset.0)
}

/// Fills `into` from `file` at `offset`, however the platform reads at a place.
#[cfg(windows)]
fn read_exact_at(
    file: &std::fs::File,
    into: RoomToFill<'_>,
    mut offset: SpillOffset,
) -> std::io::Result<()> {
    let mut into = into.0;
    while !into.is_empty() {
        let read = std::os::windows::fs::FileExt::seek_read(file, into, offset.0)?;
        if read == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        into = std::mem::take(&mut into)
            .get_mut(read..)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
        offset = SpillOffset(offset.0 + u64::try_from(read).unwrap_or(0));
    }
    Ok(())
}

impl FactsWorker for SpillWorker {
    fn resident_stability(
        &self,
    ) -> ryo_execution::inbound::use_cases::execution::types::facts::ResidentStability {
        ryo_execution::inbound::use_cases::execution::types::facts::ResidentStability::Stable
    }

    fn columns_mut(&mut self) -> Result<ColumnsMut<'_>, FactsError> {
        Ok(self
            .tables
            .iter_mut()
            .enumerate()
            .filter_map(|(place, columns)| {
                Some((all().get(place).copied()?, columns.as_mut_slice()))
            })
            .collect())
    }

    fn rows(&self, relation: Relation) -> Result<ColumnsView<'_>, FactsError> {
        let place = all()
            .iter()
            .position(|each| *each == relation)
            .ok_or_else(refused)?;
        Ok(ColumnsView {
            kinds: kinds(relation),
            columns: self.tables.get(place).ok_or_else(refused)?,
        })
    }

    fn keep(&self, bucket: Bucket, words: WordsToKeep<'_>) -> Result<(), FactsError> {
        let bytes = &words.words.0;
        if bytes.is_empty() {
            return Err(refused());
        }
        let mut spill = self.spill.lock().map_err(|_| refused())?;
        let offset = SpillOffset(
            spill
                .written
                .0
                .saturating_add(u64::try_from(spill.pending.0.len()).unwrap_or(u64::MAX)),
        );
        spill.pending.0.extend_from_slice(bytes);
        spill
            .stretches
            .entry(bucket)
            .or_default()
            .push(SpillStretch {
                offset,
                length: PackedLength(u64::try_from(bytes.len()).unwrap_or(u64::MAX)),
                file: words.file,
                proof: words.proof.clone(),
            });
        // The file is made only when what waits reaches the bound: a run that keeps little spills nothing to disk.
        if spill.pending.0.len() >= FLUSH_AT {
            if matches!(spill.file, SpillFile::Missing) {
                spill.file = SpillFile::Open(self.open()?);
            }
            flush(&mut spill)?;
        }
        Ok(())
    }

    fn release(&mut self) -> Result<(), FactsError> {
        for column in self.tables.iter_mut().flatten() {
            column.0.clear();
        }
        Ok(())
    }

    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError> {
        let mut spill = self.spill.into_inner().map_err(|_| refused())?;
        // A spill that never reached its bound keeps its words in memory: it has no file to write them to.
        if matches!(spill.file, SpillFile::Open(_)) {
            flush(&mut spill)?;
        }
        Ok(Box::new(SpillFinish {
            spilled: vec![spill],
        }))
    }
}

/// What the finish made: the spills of every worker, one after another.
struct SpillFinish {
    /// The spill of every worker, flushed, one after another.
    spilled: Vec<Spill>,
}

impl FactsFinish for SpillFinish {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        let mut words = KeptBucket::default();
        for spill in &self.spilled {
            let Some(stretches) = spill.stretches.get(&bucket) else {
                continue;
            };
            for stretch in stretches {
                let length = usize::try_from(stretch.length.0).unwrap_or(usize::MAX);
                match &spill.file {
                    SpillFile::Open(file) => {
                        let start = words.words.0.0.len();
                        words.words.0.0.resize(start.saturating_add(length), 0);
                        let room = words.words.0.0.get_mut(start..).ok_or_else(refused)?;
                        read_exact_at(file, RoomToFill(room), stretch.offset)
                            .map_err(|error| spilled(&error))?;
                        words.sources.push(BucketSource {
                            file: stretch.file,
                            start,
                            length,
                            proof: stretch.proof.clone(),
                        });
                    }
                    SpillFile::Missing => {
                        let start = usize::try_from(stretch.offset.0).unwrap_or(usize::MAX);
                        let held = spill
                            .pending
                            .0
                            .get(start..start.saturating_add(length))
                            .ok_or_else(refused)?;
                        let start = words.words.0.0.len();
                        words.words.0.0.extend_from_slice(held);
                        words.sources.push(BucketSource {
                            file: stretch.file,
                            start,
                            length,
                            proof: stretch.proof.clone(),
                        });
                    }
                }
            }
        }
        Ok(words)
    }
}

/// The finished words of every worker, one worker after another: a bucket's words are each worker's in turn.
struct Joined {
    /// What each worker finished with, in the order of the workers.
    parts: Vec<Box<dyn FactsFinish>>,
}

impl FactsFinish for Joined {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        let mut words = KeptBucket::default();
        for part in &self.parts {
            let each = part.kept(bucket)?;
            let offset = words.words.0.0.len();
            words.words.0.0.extend(each.words.0.0);
            words
                .sources
                .extend(each.sources.into_iter().map(|mut source| {
                    source.start += offset;
                    source
                }));
        }
        Ok(words)
    }
}

impl FactsPort for FactsSpillAdapter {
    fn worker(&self) -> Result<Box<dyn FactsWorker>, FactsError> {
        Ok(Box::new(SpillWorker {
            number: SpillWorkerNumber(self.next.fetch_add(1, Ordering::Relaxed)),
            directory: self.directory.clone(),
            tables: all()
                .iter()
                .map(|relation| vec![FactColumn::default(); columns(*relation).len()])
                .collect(),
            spill: Mutex::new(Spill::default()),
        }))
    }

    fn finish(
        &self,
        workers: &mut dyn Iterator<Item = Box<dyn FactsWorker>>,
    ) -> Result<Box<dyn FactsFinish>, FactsError> {
        let parts = workers
            .map(FactsWorker::finished)
            .collect::<Result<Vec<_>, FactsError>>()?;
        Ok(Box::new(Joined { parts }))
    }
}
