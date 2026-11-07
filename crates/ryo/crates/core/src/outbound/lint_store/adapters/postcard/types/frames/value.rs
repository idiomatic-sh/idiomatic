//! Compact row-span metadata and bounded copies preserve the existing postcard sequence format.

use crate::outbound::lint_store::adapters::postcard::constants::{
    BUCKET_SIZE_OVERFLOWS, COPIED_PAYLOAD_EXCEEDS_OLD_SNAPSHOT,
    FRESH_ROW_COUNT_EXCEEDS_STAGING_WIDTH, FRESH_ROW_COUNT_OVERFLOWS, HEADER_SIZE_OVERFLOWS,
    INLINE_SIZE_OVERFLOWS, INVALID_FRESH_STAGED_ROW_ORDER, MISSING_INLINE_FRESH_ROW,
    MISSING_STAGED_ROW, NO_OLD_CACHE_SNAPSHOT_WAS_READ, NO_STAGING_SESSION_WAS_READ,
    OLD_BUCKET_HAS_TRAILING_BYTES, OLD_BUCKET_HEADER_IS_TRUNCATED, OLD_BUCKET_HEADER_OVERFLOWS,
    OLD_BUCKET_SEEK_EXCEEDS_ITS_SECTION, OLD_BUCKET_SEEK_UNDERFLOWS,
    OLD_BUCKET_SPAN_IS_TRUNCATED_OR, OLD_BUCKET_VARINT_IS_TOO_LONG, OLD_FILE_NUMBER_OVERFLOWS,
    OLD_ROW_BELONGS_TO_ANOTHER_STORE, OLD_ROW_COUNT_EXCEEDS_ITS_SECTION,
    OLD_ROW_DESCRIPTOR_DOES_NOT_MATCH, OLD_ROW_EXCEEDS_ADDRESSABLE_MEMORY,
    OLD_ROW_HEADER_EXCEEDS_ADDRESSABLE_MEMORY, OLD_ROW_HEADER_EXCEEDS_ITS_ENCODED,
    OLD_ROW_HEADER_IS_INVERTED, OLD_ROW_HEADER_IS_OUTSIDE_ITS,
    OLD_ROW_PAYLOAD_EXCEEDS_ADDRESSABLE_MEMORY, OLD_ROW_PAYLOAD_EXCEEDS_ITS_SECTION,
    OLD_ROW_PAYLOAD_IS_TRUNCATED_OR, OLD_ROW_POSITION_OVERFLOWS, OLD_ROW_SEEK_UNDERFLOWS,
    PAYLOAD_EXCEEDS_POSTCARD_LENGTH_WIDTH, STORE_OFFSET_OVERFLOWS,
};
use crate::outbound::lint_store::adapters::postcard::types::spool::value::{
    BUFFER_BYTES, copy_buffered, seek_buffered,
};
use crate::outbound::lint_store::adapters::postcard::types::spool::value::{StagingSpool, invalid};
use crate::outbound::lint_store::adapters::postcard::types::{
    FileOffset, SectionLength, SectionSpan,
};
use ryo_values::domain::records::types::StagedWords;
use ryo_values::domain::stored_facts::types::{Bucket, PackedWords};
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use ryo_values::domain::stores::types::{OldWords, StagedRow, StoreSnapshot, StoredWords};
use std::collections::BTreeMap;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};

#[cfg(unix)]
struct PositionalReader<'a> {
    file: &'a std::fs::File,
    at: u64,
}

#[cfg(unix)]
impl Read for PositionalReader<'_> {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        use std::os::unix::fs::FileExt as _;

        let read = self.file.read_at(into, self.at)?;
        self.at = self
            .at
            .checked_add(u64::try_from(read).map_err(std::io::Error::other)?)
            .ok_or_else(|| invalid(OLD_ROW_POSITION_OVERFLOWS))?;
        Ok(read)
    }
}

#[cfg(unix)]
impl Seek for PositionalReader<'_> {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let at = match from {
            SeekFrom::Start(at) => i128::from(at),
            SeekFrom::Current(by) => i128::from(self.at) + i128::from(by),
            SeekFrom::End(by) => i128::from(self.file.metadata()?.len()) + i128::from(by),
        };
        self.at = u64::try_from(at).map_err(|_| invalid(OLD_ROW_SEEK_UNDERFLOWS))?;
        Ok(self.at)
    }
}

/// One borrowed or file-backed payload, never an eagerly allocated rebuilt bucket.
#[derive(Clone, Copy, Debug)]
pub(in crate::outbound::lint_store::adapters::postcard) enum Payload<'a> {
    /// Only adapters explicitly retaining inline data use this arm.
    Inline(&'a [u8]),
    /// A fresh payload belonging to this exact run's temporary spool.
    Staged(StagedWords),
    /// A selected row in the immutable old cache snapshot.
    Old(SectionSpan),
}

/// File identity plus the original encounter order in its bucket vector.
#[derive(Clone, Copy, Debug)]
pub(in crate::outbound::lint_store::adapters::postcard) struct Frame<'a> {
    /// Final file number to serialize.
    pub file: StoredFileNumber,
    /// Original packed bytes or an exact byte span.
    pub payload: Payload<'a>,
}

/// One validated old row, including the framing offset needed by lazy snapshot reads.
#[derive(Clone, Copy, Debug)]
pub(in crate::outbound::lint_store::adapters::postcard) struct OldFrame {
    /// Original file number, unchanged until application remapping.
    pub file: StoredFileNumber,
    /// Header start, before its two postcard integers.
    pub header: u64,
    /// Original packed bytes, left in the immutable open snapshot.
    pub span: SectionSpan,
}

/// Cap header read-ahead at the requested section; neighboring buckets remain unread.
struct BucketReader<'a, R> {
    file: &'a mut R,
    start: u64,
    end: u64,
    at: u64,
}

impl<R: Read> Read for BucketReader<'_, R> {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        let count = into
            .len()
            .min(usize::try_from(self.end - self.at).unwrap_or(usize::MAX));
        let read = self.file.read(&mut into[..count])?;
        self.at += u64::try_from(read).map_err(std::io::Error::other)?;
        Ok(read)
    }
}

impl<R: Seek> Seek for BucketReader<'_, R> {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let at = match from {
            SeekFrom::Start(at) => i128::from(at),
            SeekFrom::Current(by) => i128::from(self.at) + i128::from(by),
            SeekFrom::End(by) => i128::from(self.end) + i128::from(by),
        };
        let at = u64::try_from(at).map_err(|_| invalid(OLD_BUCKET_SEEK_UNDERFLOWS))?;
        if at < self.start || at > self.end {
            return Err(invalid(OLD_BUCKET_SEEK_EXCEEDS_ITS_SECTION));
        }
        self.at = self.file.seek(SeekFrom::Start(at))?;
        Ok(self.at)
    }
}

/// Restore one fresh sequence from inline entries and staged ordinal slots; no payload is copied.
pub(in crate::outbound::lint_store::adapters::postcard) fn fresh<'a>(
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
                payload: Payload::Inline(&row.words.0),
            });
        }
    }
    Ok(buckets)
}

/// Parse original rows for the unchanged persistent bucket-copy path.
pub(in crate::outbound::lint_store::adapters::postcard) fn old_rows(
    file: &mut std::fs::File,
    span: SectionSpan,
) -> std::io::Result<Vec<Frame<'static>>> {
    let held = file.metadata()?.len();
    old_rows_in(file, span, held)
}

/// The prior frame view remains byte-compatible for writers and their work tests.
pub(in crate::outbound::lint_store::adapters::postcard) fn old_rows_in(
    file: &mut (impl Read + Seek),
    span: SectionSpan,
    held: u64,
) -> std::io::Result<Vec<Frame<'static>>> {
    Ok(old_word_rows_in(file, span, held)?
        .into_iter()
        .map(|row| Frame {
            file: row.file,
            payload: Payload::Old(row.span),
        })
        .collect())
}

/// Enumerate header descriptors using bounded read-ahead, never a whole bucket allocation.
pub(in crate::outbound::lint_store::adapters::postcard) fn old_word_rows(
    file: &mut std::fs::File,
    span: SectionSpan,
) -> std::io::Result<Vec<OldFrame>> {
    let held = file.metadata()?.len();
    old_word_rows_in(file, span, held)
}

pub(in crate::outbound::lint_store::adapters::postcard) fn old_word_rows_in(
    file: &mut (impl Read + Seek),
    span: SectionSpan,
    held: u64,
) -> std::io::Result<Vec<OldFrame>> {
    let end = span
        .start
        .0
        .checked_add(span.length.0)
        .filter(|end| *end <= held)
        .ok_or_else(|| invalid(OLD_BUCKET_SPAN_IS_TRUNCATED_OR))?;
    let _position = file.seek(SeekFrom::Start(span.start.0))?;
    let bounded = BucketReader {
        file,
        start: span.start.0,
        end,
        at: span.start.0,
    };
    let mut file = BufReader::with_capacity(BUFFER_BYTES, bounded);
    let mut cursor = span.start.0;
    let count = integer(
        &mut file,
        &mut cursor,
        end,
        u64::try_from(usize::MAX).unwrap_or(u64::MAX),
    )?;
    if count > span.length.0 / 2 {
        return Err(invalid(OLD_ROW_COUNT_EXCEEDS_ITS_SECTION));
    }
    let mut rows = Vec::new();
    for _ in 0..count {
        let header = cursor;
        let number = integer(&mut file, &mut cursor, end, u64::from(u32::MAX))?;
        let length = integer(
            &mut file,
            &mut cursor,
            end,
            u64::try_from(usize::MAX).unwrap_or(u64::MAX),
        )?;
        let after = cursor
            .checked_add(length)
            .filter(|after| *after <= end)
            .ok_or_else(|| invalid(OLD_ROW_PAYLOAD_EXCEEDS_ITS_SECTION))?;
        rows.try_reserve(1).map_err(std::io::Error::other)?;
        rows.push(OldFrame {
            file: StoredFileNumber(
                u32::try_from(number).map_err(|_| invalid(OLD_FILE_NUMBER_OVERFLOWS))?,
            ),
            header,
            span: SectionSpan {
                start: FileOffset(cursor),
                length: SectionLength(length),
            },
        });
        seek_buffered(&mut file, cursor, after)?;
        cursor = after;
    }
    if cursor != end {
        return Err(invalid(OLD_BUCKET_HAS_TRAILING_BYTES));
    }
    Ok(rows)
}

/// Resolve one row from an already-open immutable snapshot after validating its exact framing.
/// No buffering here can pull another row's payload into the caller's read.
pub(in crate::outbound::lint_store::adapters::postcard) fn read_old_in(
    file: &mut (impl Read + Seek),
    held: u64,
    snapshot: StoreSnapshot,
    bucket: SectionSpan,
    words: &OldWords,
    into: &mut PackedWords,
) -> std::io::Result<()> {
    if words.snapshot != snapshot {
        return Err(invalid(OLD_ROW_BELONGS_TO_ANOTHER_STORE));
    }
    let end = bucket
        .start
        .0
        .checked_add(bucket.length.0)
        .filter(|end| *end <= held)
        .ok_or_else(|| invalid(OLD_BUCKET_SPAN_IS_TRUNCATED_OR))?;
    let after = words
        .offset
        .0
        .checked_add(words.length.0)
        .filter(|after| *after <= end)
        .ok_or_else(|| invalid(OLD_ROW_PAYLOAD_IS_TRUNCATED_OR))?;
    if words.header.0 <= bucket.start.0
        || words.header.0 >= words.offset.0
        || words.offset.0 > after
    {
        return Err(invalid(OLD_ROW_HEADER_IS_OUTSIDE_ITS));
    }
    let length = usize::try_from(words.length.0)
        .map_err(|_| invalid(OLD_ROW_PAYLOAD_EXCEEDS_ADDRESSABLE_MEMORY))?;
    // Only the two encoded fields belong to this row header. Reading exactly that
    // span avoids byte-at-a-time file reads without prefetching another payload.
    const HEADER_BYTES: usize = 5 + (size_of::<usize>() * 8).div_ceil(7);
    let header_length = words
        .offset
        .0
        .checked_sub(words.header.0)
        .ok_or_else(|| invalid(OLD_ROW_HEADER_IS_INVERTED))?;
    let header_length = usize::try_from(header_length)
        .map_err(|_| invalid(OLD_ROW_HEADER_EXCEEDS_ADDRESSABLE_MEMORY))?;
    if !(2..=HEADER_BYTES).contains(&header_length) {
        return Err(invalid(OLD_ROW_HEADER_EXCEEDS_ITS_ENCODED));
    }
    let total = header_length
        .checked_add(length)
        .ok_or_else(|| invalid(OLD_ROW_EXCEEDS_ADDRESSABLE_MEMORY))?;
    into.0.clear();
    into.0
        .try_reserve_exact(total)
        .map_err(std::io::Error::other)?;
    into.0.resize(total, 0);
    let _position = file.seek(SeekFrom::Start(words.header.0))?;
    if let Err(error) = file.read_exact(&mut into.0) {
        into.0.clear();
        return Err(error);
    }
    let framing = (|| {
        let mut fields = std::io::Cursor::new(&into.0[..header_length]);
        let mut cursor = words.header.0;
        let number = integer(
            &mut fields,
            &mut cursor,
            words.offset.0,
            u64::from(u32::MAX),
        )?;
        let framed_length = integer(
            &mut fields,
            &mut cursor,
            words.offset.0,
            u64::try_from(usize::MAX).unwrap_or(u64::MAX),
        )?;
        Ok::<_, std::io::Error>((number, framed_length, cursor))
    })()
    .map_err(|error| {
        into.0.clear();
        error
    })?;
    let (number, framed_length, cursor) = framing;
    if number != u64::from(words.file.0)
        || framed_length != words.length.0
        || cursor != words.offset.0
    {
        into.0.clear();
        return Err(invalid(OLD_ROW_DESCRIPTOR_DOES_NOT_MATCH));
    }
    into.0.copy_within(header_length.., 0);
    into.0.truncate(length);
    Ok(())
}

/// Resolve an old row without changing the shared snapshot file cursor. This keeps
/// the exact requested-row read and late truncation checks of `read_old_in`.
#[cfg(unix)]
pub(in crate::outbound::lint_store::adapters::postcard) fn read_old_file_in(
    file: &std::fs::File,
    held: u64,
    snapshot: StoreSnapshot,
    bucket: SectionSpan,
    words: &OldWords,
    into: &mut PackedWords,
) -> std::io::Result<()> {
    read_old_in(
        &mut PositionalReader { file, at: 0 },
        held,
        snapshot,
        bucket,
        words,
        into,
    )
}

/// Postcard unsigned varints are bounded by the encoded type's width, including a checked last digit.
fn integer(file: &mut impl Read, cursor: &mut u64, end: u64, maximum: u64) -> std::io::Result<u64> {
    let digits = ((64 - maximum.leading_zeros()).max(1)).div_ceil(7);
    let mut value = 0_u64;
    for digit in 0..digits {
        if *cursor >= end {
            return Err(invalid(OLD_BUCKET_HEADER_IS_TRUNCATED));
        }
        let mut byte = [0_u8; 1];
        file.read_exact(&mut byte)?;
        *cursor += 1;
        let payload = u64::from(byte[0] & 127);
        let shift = digit * 7;
        if payload > maximum >> shift {
            return Err(invalid(OLD_BUCKET_HEADER_OVERFLOWS));
        }
        value |= payload << shift;
        if byte[0] & 128 == 0 {
            return Ok(value);
        }
    }
    Err(invalid(OLD_BUCKET_VARINT_IS_TOO_LONG))
}

/// Serialize numbers with the SAME codec used by the prior whole-sequence serializer.
fn header<T: serde::Serialize>(into: &mut impl Write, value: &T) -> std::io::Result<u64> {
    let mut buffer = [0_u8; 16];
    let bytes = postcard::to_slice(value, &mut buffer).map_err(std::io::Error::other)?;
    into.write_all(bytes)?;
    u64::try_from(bytes.len()).map_err(|_| invalid(HEADER_SIZE_OVERFLOWS))
}

/// Write one existing-format bucket, streaming every selected payload without a rebuilt Vec.
pub(in crate::outbound::lint_store::adapters::postcard) fn write_bucket(
    into: &mut impl Write,
    at: &mut FileOffset,
    rows: &[Frame<'_>],
    mut old: Option<&mut std::fs::File>,
    mut staged: Option<&mut StagingSpool>,
) -> std::io::Result<SectionSpan> {
    // Snapshot size and position are validated once per bucket, not once per tiny row.
    let old_length = old
        .as_ref()
        .map(|file| file.metadata().map(|metadata| metadata.len()))
        .transpose()?;
    let mut old_position = old
        .as_deref_mut()
        .map(|file| file.stream_position())
        .transpose()?
        .unwrap_or(0);
    let mut old = old.map(|file| BufReader::with_capacity(BUFFER_BYTES, file));
    if let Some(spool) = staged.as_deref_mut() {
        spool.flush()?;
    }
    let start = *at;
    let mut length = header(into, &rows.len())?;
    for row in rows {
        let size = match row.payload {
            Payload::Inline(bytes) => {
                u64::try_from(bytes.len()).map_err(|_| invalid(INLINE_SIZE_OVERFLOWS))?
            }
            Payload::Staged(span) => span.length.0,
            Payload::Old(span) => span.length.0,
        };
        let addressable =
            usize::try_from(size).map_err(|_| invalid(PAYLOAD_EXCEEDS_POSTCARD_LENGTH_WIDTH))?;
        let file_header = header(into, &row.file.0)?;
        let size_header = header(into, &addressable)?;
        length = length
            .checked_add(file_header)
            .and_then(|length| length.checked_add(size_header))
            .ok_or_else(|| invalid(BUCKET_SIZE_OVERFLOWS))?;
        match row.payload {
            Payload::Inline(bytes) => into.write_all(bytes)?,
            Payload::Staged(span) => staged
                .as_deref_mut()
                .ok_or_else(|| invalid(NO_STAGING_SESSION_WAS_READ))?
                .copy(&span, into)?,
            Payload::Old(span) => {
                let end = span
                    .start
                    .0
                    .checked_add(span.length.0)
                    .filter(|end| old_length.is_some_and(|held| *end <= held))
                    .ok_or_else(|| invalid(COPIED_PAYLOAD_EXCEEDS_OLD_SNAPSHOT))?;
                let reader = old
                    .as_mut()
                    .ok_or_else(|| invalid(NO_OLD_CACHE_SNAPSHOT_WAS_READ))?;
                if old_position != span.start.0 {
                    seek_buffered(reader, old_position, span.start.0)?;
                }
                copy_buffered(reader, into, span.length.0)?;
                old_position = end;
            }
        }
        length = length
            .checked_add(size)
            .ok_or_else(|| invalid(BUCKET_SIZE_OVERFLOWS))?;
    }
    at.0 =
        at.0.checked_add(length)
            .ok_or_else(|| invalid(STORE_OFFSET_OVERFLOWS))?;
    Ok(SectionSpan {
        start,
        length: SectionLength(length),
    })
}
