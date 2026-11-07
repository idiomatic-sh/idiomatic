//! Temporary-file ownership stays in the adapter; descriptors contain only a session and checked span.

use crate::outbound::lint_store::adapters::postcard::constants::COPIED_PAYLOAD_IS_TRUNCATED;
use crate::outbound::lint_store::adapters::postcard::constants::COPY_SIZE_OVERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::FOREIGN_OR_STALE_STAGING_SESSION;
use crate::outbound::lint_store::adapters::postcard::constants::MISSING_SPOOL_READER;
use crate::outbound::lint_store::adapters::postcard::constants::MISSING_SPOOL_WRITER;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_BUCKET_SEEK_EXCEEDS_ITS_SECTION;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_BUCKET_SEEK_UNDERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_POSITION_OVERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_SEEK_UNDERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::STAGED_OFFSET_OVERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::STAGED_PAYLOAD_EXCEEDS_ADDRESSABLE_MEMORY;
use crate::outbound::lint_store::adapters::postcard::constants::STAGED_PAYLOAD_LENGTH_OVERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::STAGED_SPAN_IS_OUTSIDE_THIS_SNAPSHOT;
use crate::outbound::lint_store::adapters::postcard::types::FileOffset;
use ryo_values::domain::records::types::PayloadLength;
use ryo_values::domain::records::types::PayloadOffset;
use ryo_values::domain::records::types::StagedWords;
use ryo_values::domain::records::types::StagingSession;
use ryo_values::domain::stored_facts::types::PackedWords;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

/// One RAII temporary file, shared only through its ReadStore's mutex.
#[derive(Debug)]
pub(super) struct StagingSpool {
    /// NamedTempFile closes its handle before removing the temporary path on every platform.
    pub file: tempfile::NamedTempFile,
    /// Unique within the process; descriptors never survive this snapshot.
    pub session: StagingSession,
    /// Logically accepted bytes, flushed before every replay/publication boundary.
    pub length: PayloadLength,
    writer: Option<BufWriter<std::fs::File>>,
    reader: Option<BufReader<std::fs::File>>,
    read_at: PayloadOffset,
}

impl StagingSpool {
    /// Creation is deferred until the first fresh normalized bucket.
    pub(super) fn new(
        session: StagingSession,
    ) -> std::io::Result<Self> {
        Self::create(session, tempfile::NamedTempFile::new)
    }

    /// The creation seam is local to this adapter; tests can inject failure without global state.
    pub(super) fn create(
        session: StagingSession,
        create: impl FnOnce() -> std::io::Result<tempfile::NamedTempFile>,
    ) -> std::io::Result<Self> {
        Ok(Self {
            file: create()?,
            session,
            length: PayloadLength(0),
            writer: None,
            reader: None,
            read_at: PayloadOffset(0),
        })
    }

    /// Append without retaining the caller's PackedWords allocation.
    pub(super) fn append(
        &mut self,
        words: PackedWords,
    ) -> std::io::Result<StagedWords> {
        let length =
            u64::try_from(words.0.len()).map_err(|_| invalid(STAGED_PAYLOAD_LENGTH_OVERFLOWS))?;
        let end = self
            .length
            .0
            .checked_add(length)
            .ok_or_else(|| invalid(STAGED_OFFSET_OVERFLOWS))?;
        if self.writer.is_none() {
            self.reader = None;
            let mut writer =
                BufWriter::with_capacity(BUFFER_BYTES, self.file.as_file().try_clone()?);
            let _position = writer.seek(SeekFrom::Start(self.length.0))?;
            self.writer = Some(writer);
        }
        if let Err(error) = self
            .writer
            .as_mut()
            .ok_or_else(|| invalid(MISSING_SPOOL_WRITER))?
            .write_all(&words.0)
        {
            self.writer = None;
            return Err(error);
        }
        let span = StagedWords {
            session: self.session,
            offset: ryo_values::domain::records::types::PayloadOffset(self.length.0),
            length: ryo_values::domain::records::types::PayloadLength(length),
        };
        self.length = PayloadLength(end);
        Ok(span)
    }

    /// Only a checked span from this exact snapshot can be read.
    pub(super) fn read(
        &mut self,
        span: &StagedWords,
        into: &mut PackedWords,
    ) -> std::io::Result<()> {
        validate(self.session, self.length, span)?;
        let length = usize::try_from(span.length.0)
            .map_err(|_| invalid(STAGED_PAYLOAD_EXCEEDS_ADDRESSABLE_MEMORY))?;
        self.position(span.offset)?;
        into.0.clear();
        into.0
            .try_reserve_exact(length)
            .map_err(std::io::Error::other)?;
        into.0.resize(length, 0);
        if let Err(error) = self
            .reader
            .as_mut()
            .ok_or_else(|| invalid(MISSING_SPOOL_READER))?
            .read_exact(&mut into.0)
        {
            self.reader = None;
            return Err(error);
        }
        self.read_at = PayloadOffset(span.offset.0 + span.length.0);
        Ok(())
    }

    /// Stream a checked payload using bounded std IO scratch, never a whole-bucket Vec.
    pub(super) fn copy<W: Write>(
        &mut self,
        span: &StagedWords,
        into: &mut W,
    ) -> std::io::Result<()> {
        validate(self.session, self.length, span)?;
        self.position(span.offset)?;
        if let Err(error) = copy_buffered(
            self.reader
                .as_mut()
                .ok_or_else(|| invalid(MISSING_SPOOL_READER))?,
            into,
            span.length,
        ) {
            self.reader = None;
            return Err(error);
        }
        self.read_at = PayloadOffset(span.offset.0 + span.length.0);
        Ok(())
    }

    /// Flush is a required error barrier before replay/publication, never an fsync promise.
    pub(super) fn flush(
        &mut self,
    ) -> std::io::Result<()> {
        if let Some(writer) = &mut self.writer {
            writer.flush()?;
        }
        Ok(())
    }

    /// Preserve buffered contiguous reads; only discontinuities reposition the underlying file.
    fn position(&mut self, offset: PayloadOffset) -> std::io::Result<()> {
        self.flush()?;
        if self.reader.is_none() {
            self.writer = None;
            let mut reader =
                BufReader::with_capacity(BUFFER_BYTES, self.file.as_file().try_clone()?);
            let _position = reader.seek(SeekFrom::Start(offset.0))?;
            self.reader = Some(reader);
            self.read_at = offset;
        } else if offset != self.read_at {
            let reader = self
                .reader
                .as_mut()
                .ok_or_else(|| invalid(MISSING_SPOOL_READER))?;
            seek_buffered(reader, self.read_at, offset)?;
            self.read_at = offset;
        }
        Ok(())
    }
}

// Close cloned IO handles before NamedTempFile removes its path, including on Windows.
impl Drop for StagingSpool {
    fn drop(&mut self) {
        self.writer = None;
        self.reader = None;
    }
}

/// Invalid metadata is an IO refusal, never an empty row or a saturated offset.
pub(super) fn invalid(reason: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, reason.to_string())
}

/// Arithmetic and session checks precede allocation, seek or read.
pub(super) fn validate(
    session: StagingSession,
    held: PayloadLength,
    span: &StagedWords,
) -> std::io::Result<()> {
    if span.session != session {
        return Err(invalid(FOREIGN_OR_STALE_STAGING_SESSION));
    }
    if span
        .offset
        .0
        .checked_add(span.length.0)
        .is_none_or(|end| end > held.0)
    {
        return Err(invalid(STAGED_SPAN_IS_OUTSIDE_THIS_SNAPSHOT));
    }
    Ok(())
}

/// Standard-IO seams keep failure tests independent of operating-system permissions.
#[cfg(test)]
pub(super) fn append<F: Write + Seek>(
    file: &mut F,
    session: StagingSession,
    held: &mut PayloadLength,
    words: &PackedWords,
) -> std::io::Result<StagedWords> {
    let length =
        u64::try_from(words.0.len()).map_err(|_| invalid(STAGED_PAYLOAD_LENGTH_OVERFLOWS))?;
    let end = held
        .0
        .checked_add(length)
        .ok_or_else(|| invalid(STAGED_OFFSET_OVERFLOWS))?;
    let _position = file.seek(SeekFrom::Start(held.0))?;
    file.write_all(&words.0)?;
    // Commit logical length only after every append/flush operation succeeds.
    file.flush()?;
    let span = StagedWords {
        session,
        offset: ryo_values::domain::records::types::PayloadOffset(held.0),
        length: ryo_values::domain::records::types::PayloadLength(length),
    };
    *held = PayloadLength(end);
    Ok(span)
}

/// At most one selected normalized bucket is owned by the replay caller.
#[cfg(test)]
pub(super) fn read<F: Read + Write + Seek>(
    file: &mut F,
    session: StagingSession,
    held: PayloadLength,
    span: &StagedWords,
    into: &mut PackedWords,
) -> std::io::Result<()> {
    validate(session, held, span)?;
    let length = usize::try_from(span.length.0)
        .map_err(|_| invalid(STAGED_PAYLOAD_EXCEEDS_ADDRESSABLE_MEMORY))?;
    file.flush()?;
    let _position = file.seek(SeekFrom::Start(span.offset.0))?;
    into.0.clear();
    into.0
        .try_reserve_exact(length)
        .map_err(std::io::Error::other)?;
    into.0.resize(length, 0);
    file.read_exact(&mut into.0)
}

/// Fixed IO memory per active snapshot/stream, independent of row count/payload size.
pub(super) const BUFFER_BYTES: usize = 64 * 1024;

/// Skip within the read buffer where possible; contiguous spans need no seek syscall.
pub(super) fn seek_buffered(
    reader: &mut (impl BufRead + Seek),
    current: PayloadOffset,
    target: PayloadOffset,
) -> std::io::Result<()> {
    let delta = i128::from(target.0) - i128::from(current.0);
    if let Ok(delta) = i64::try_from(delta) {
        reader.seek_relative(delta)
    } else {
        reader.seek(SeekFrom::Start(target.0)).map(|_| ())
    }
}

/// Copy from existing read buffers; never dispatch per-row file-copy syscalls.
pub(super) fn copy_buffered(
    reader: &mut impl BufRead,
    into: &mut impl Write,
    mut length: PayloadLength,
) -> std::io::Result<()> {
    while length.0 != 0 {
        let bytes = reader.fill_buf()?;
        if bytes.is_empty() {
            return Err(invalid(COPIED_PAYLOAD_IS_TRUNCATED));
        }
        let count = bytes
            .len()
            .min(usize::try_from(length.0).unwrap_or(usize::MAX));
        into.write_all(&bytes[..count])?;
        reader.consume(count);
        length.0 -= u64::try_from(count).map_err(|_| invalid(COPY_SIZE_OVERFLOWS))?;
    }
    Ok(())
}

/// Reads the old snapshot at a position of its own, leaving the file's own cursor where it is.
pub(super) struct PositionalReader<'a> {
    pub file: &'a std::fs::File,
    pub at: FileOffset,
}

#[cfg(unix)]
impl Read for PositionalReader<'_> {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        use std::os::unix::fs::FileExt as _;

        let read = self.file.read_at(into, self.at.0)?;
        self.at = FileOffset(
            self.at
                .0
                .checked_add(u64::try_from(read).map_err(std::io::Error::other)?)
                .ok_or_else(|| invalid(OLD_ROW_POSITION_OVERFLOWS))?,
        );
        Ok(read)
    }
}

#[cfg(not(unix))]
impl Read for PositionalReader<'_> {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        let mut file = self.file;
        let _position = file.seek(SeekFrom::Start(self.at.0))?;
        let read = file.read(into)?;
        self.at = FileOffset(
            self.at
                .0
                .checked_add(u64::try_from(read).map_err(std::io::Error::other)?)
                .ok_or_else(|| invalid(OLD_ROW_POSITION_OVERFLOWS))?,
        );
        Ok(read)
    }
}

impl Seek for PositionalReader<'_> {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let at = match from {
            SeekFrom::Start(at) => i128::from(at),
            SeekFrom::Current(by) => i128::from(self.at.0) + i128::from(by),
            SeekFrom::End(by) => i128::from(self.file.metadata()?.len()) + i128::from(by),
        };
        self.at = FileOffset(u64::try_from(at).map_err(|_| invalid(OLD_ROW_SEEK_UNDERFLOWS))?);
        Ok(self.at.0)
    }
}

/// Cap header read-ahead at the requested section; neighboring buckets remain unread.
pub(super) struct BucketReader<'a, R> {
    pub file: &'a mut R,
    pub start: FileOffset,
    pub end: FileOffset,
    pub at: FileOffset,
}

impl<R: Read> Read for BucketReader<'_, R> {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        let count = into
            .len()
            .min(usize::try_from(self.end.0 - self.at.0).unwrap_or(usize::MAX));
        let read = self.file.read(&mut into[..count])?;
        self.at.0 += u64::try_from(read).map_err(std::io::Error::other)?;
        Ok(read)
    }
}

impl<R: Seek> Seek for BucketReader<'_, R> {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let at = match from {
            SeekFrom::Start(at) => i128::from(at),
            SeekFrom::Current(by) => i128::from(self.at.0) + i128::from(by),
            SeekFrom::End(by) => i128::from(self.end.0) + i128::from(by),
        };
        let at = u64::try_from(at).map_err(|_| invalid(OLD_BUCKET_SEEK_UNDERFLOWS))?;
        if at < self.start.0 || at > self.end.0 {
            return Err(invalid(OLD_BUCKET_SEEK_EXCEEDS_ITS_SECTION));
        }
        self.at = FileOffset(self.file.seek(SeekFrom::Start(at))?);
        Ok(self.at.0)
    }
}
