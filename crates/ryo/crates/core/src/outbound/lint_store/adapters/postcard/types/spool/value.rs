//! Temporary-file ownership stays in the adapter; descriptors contain only a session and checked span.

use crate::outbound::lint_store::adapters::postcard::constants::{
    COPIED_PAYLOAD_IS_TRUNCATED, COPY_SIZE_OVERFLOWS, FOREIGN_OR_STALE_STAGING_SESSION,
    MISSING_SPOOL_READER, MISSING_SPOOL_WRITER, STAGED_OFFSET_OVERFLOWS,
    STAGED_PAYLOAD_EXCEEDS_ADDRESSABLE_MEMORY, STAGED_PAYLOAD_LENGTH_OVERFLOWS,
    STAGED_SPAN_IS_OUTSIDE_THIS_SNAPSHOT,
};
use ryo_values::domain::records::types::{StagedWords, StagingSession};
use ryo_values::domain::stored_facts::types::PackedWords;
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};

/// One RAII temporary file, shared only through its ReadStore's mutex.
#[derive(Debug)]
pub(in crate::outbound::lint_store::adapters::postcard) struct StagingSpool {
    /// NamedTempFile closes its handle before removing the temporary path on every platform.
    pub file: tempfile::NamedTempFile,
    /// Unique within the process; descriptors never survive this snapshot.
    pub session: StagingSession,
    /// Logically accepted bytes, flushed before every replay/publication boundary.
    pub length: u64,
    writer: Option<BufWriter<std::fs::File>>,
    reader: Option<BufReader<std::fs::File>>,
    read_at: u64,
}

impl StagingSpool {
    /// Creation is deferred until the first fresh normalized bucket.
    pub(in crate::outbound::lint_store::adapters::postcard) fn new(
        session: StagingSession,
    ) -> std::io::Result<Self> {
        Self::create(session, tempfile::NamedTempFile::new)
    }

    /// The creation seam is local to this adapter; tests can inject failure without global state.
    pub(in crate::outbound::lint_store::adapters::postcard) fn create(
        session: StagingSession,
        create: impl FnOnce() -> std::io::Result<tempfile::NamedTempFile>,
    ) -> std::io::Result<Self> {
        Ok(Self {
            file: create()?,
            session,
            length: 0,
            writer: None,
            reader: None,
            read_at: 0,
        })
    }

    /// Append without retaining the caller's PackedWords allocation.
    pub(in crate::outbound::lint_store::adapters::postcard) fn append(
        &mut self,
        words: PackedWords,
    ) -> std::io::Result<StagedWords> {
        let length =
            u64::try_from(words.0.len()).map_err(|_| invalid(STAGED_PAYLOAD_LENGTH_OVERFLOWS))?;
        let end = self
            .length
            .checked_add(length)
            .ok_or_else(|| invalid(STAGED_OFFSET_OVERFLOWS))?;
        if self.writer.is_none() {
            self.reader = None;
            let mut writer =
                BufWriter::with_capacity(BUFFER_BYTES, self.file.as_file().try_clone()?);
            let _position = writer.seek(SeekFrom::Start(self.length))?;
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
            offset: ryo_values::domain::records::types::PayloadOffset(self.length),
            length: ryo_values::domain::records::types::PayloadLength(length),
        };
        self.length = end;
        Ok(span)
    }

    /// Only a checked span from this exact snapshot can be read.
    pub(in crate::outbound::lint_store::adapters::postcard) fn read(
        &mut self,
        span: &StagedWords,
        into: &mut PackedWords,
    ) -> std::io::Result<()> {
        validate(self.session, self.length, span)?;
        let length = usize::try_from(span.length.0)
            .map_err(|_| invalid(STAGED_PAYLOAD_EXCEEDS_ADDRESSABLE_MEMORY))?;
        self.position(span.offset.0)?;
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
        self.read_at = span.offset.0 + span.length.0;
        Ok(())
    }

    /// Stream a checked payload using bounded std IO scratch, never a whole-bucket Vec.
    pub(in crate::outbound::lint_store::adapters::postcard) fn copy<W: Write>(
        &mut self,
        span: &StagedWords,
        into: &mut W,
    ) -> std::io::Result<()> {
        validate(self.session, self.length, span)?;
        self.position(span.offset.0)?;
        if let Err(error) = copy_buffered(
            self.reader
                .as_mut()
                .ok_or_else(|| invalid(MISSING_SPOOL_READER))?,
            into,
            span.length.0,
        ) {
            self.reader = None;
            return Err(error);
        }
        self.read_at = span.offset.0 + span.length.0;
        Ok(())
    }

    /// Flush is a required error barrier before replay/publication, never an fsync promise.
    pub(in crate::outbound::lint_store::adapters::postcard) fn flush(
        &mut self,
    ) -> std::io::Result<()> {
        if let Some(writer) = &mut self.writer {
            writer.flush()?;
        }
        Ok(())
    }

    /// Preserve buffered contiguous reads; only discontinuities reposition the underlying file.
    fn position(&mut self, offset: u64) -> std::io::Result<()> {
        self.flush()?;
        if self.reader.is_none() {
            self.writer = None;
            let mut reader =
                BufReader::with_capacity(BUFFER_BYTES, self.file.as_file().try_clone()?);
            let _position = reader.seek(SeekFrom::Start(offset))?;
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
pub(in crate::outbound::lint_store::adapters::postcard) fn invalid(
    reason: &'static str,
) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, reason)
}

/// Arithmetic and session checks precede allocation, seek or read.
pub(in crate::outbound::lint_store::adapters::postcard) fn validate(
    session: StagingSession,
    held: u64,
    span: &StagedWords,
) -> std::io::Result<()> {
    if span.session != session {
        return Err(invalid(FOREIGN_OR_STALE_STAGING_SESSION));
    }
    if span
        .offset
        .0
        .checked_add(span.length.0)
        .is_none_or(|end| end > held)
    {
        return Err(invalid(STAGED_SPAN_IS_OUTSIDE_THIS_SNAPSHOT));
    }
    Ok(())
}

/// Standard-IO seams keep failure tests independent of operating-system permissions.
#[cfg(test)]
pub(in crate::outbound::lint_store::adapters::postcard) fn append<F: Write + Seek>(
    file: &mut F,
    session: StagingSession,
    held: &mut u64,
    words: &PackedWords,
) -> std::io::Result<StagedWords> {
    let length =
        u64::try_from(words.0.len()).map_err(|_| invalid(STAGED_PAYLOAD_LENGTH_OVERFLOWS))?;
    let end = held
        .checked_add(length)
        .ok_or_else(|| invalid(STAGED_OFFSET_OVERFLOWS))?;
    let _position = file.seek(SeekFrom::Start(*held))?;
    file.write_all(&words.0)?;
    // Commit logical length only after every append/flush operation succeeds.
    file.flush()?;
    let span = StagedWords {
        session,
        offset: ryo_values::domain::records::types::PayloadOffset(*held),
        length: ryo_values::domain::records::types::PayloadLength(length),
    };
    *held = end;
    Ok(span)
}

/// At most one selected normalized bucket is owned by the replay caller.
#[cfg(test)]
pub(in crate::outbound::lint_store::adapters::postcard) fn read<F: Read + Write + Seek>(
    file: &mut F,
    session: StagingSession,
    held: u64,
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
pub(in crate::outbound::lint_store::adapters::postcard) const BUFFER_BYTES: usize = 64 * 1024;

/// Skip within the read buffer where possible; contiguous spans need no seek syscall.
pub(in crate::outbound::lint_store::adapters::postcard) fn seek_buffered(
    reader: &mut (impl BufRead + Seek),
    current: u64,
    target: u64,
) -> std::io::Result<()> {
    let delta = i128::from(target) - i128::from(current);
    if let Ok(delta) = i64::try_from(delta) {
        reader.seek_relative(delta)
    } else {
        reader.seek(SeekFrom::Start(target)).map(|_| ())
    }
}

/// Copy from existing read buffers; never dispatch per-row file-copy syscalls.
pub(in crate::outbound::lint_store::adapters::postcard) fn copy_buffered(
    reader: &mut impl BufRead,
    into: &mut impl Write,
    mut length: u64,
) -> std::io::Result<()> {
    while length != 0 {
        let bytes = reader.fill_buf()?;
        if bytes.is_empty() {
            return Err(invalid(COPIED_PAYLOAD_IS_TRUNCATED));
        }
        let count = bytes
            .len()
            .min(usize::try_from(length).unwrap_or(usize::MAX));
        into.write_all(&bytes[..count])?;
        reader.consume(count);
        length -= u64::try_from(count).map_err(|_| invalid(COPY_SIZE_OVERFLOWS))?;
    }
    Ok(())
}
