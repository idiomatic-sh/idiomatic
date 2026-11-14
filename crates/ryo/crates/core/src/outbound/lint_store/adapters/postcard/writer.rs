//! Compact row-span metadata and bounded copies preserve the existing postcard sequence format.

use crate::outbound::lint_store::adapters::postcard::connection::BUFFER_BYTES;
use crate::outbound::lint_store::adapters::postcard::connection::BucketReader;
use crate::outbound::lint_store::adapters::postcard::connection::StagingConnection;
use crate::outbound::lint_store::adapters::postcard::connection::StoreFile;
use crate::outbound::lint_store::adapters::postcard::connection::copy_buffered;
use crate::outbound::lint_store::adapters::postcard::connection::invalid;
use crate::outbound::lint_store::adapters::postcard::connection::seek_buffered;
use crate::outbound::lint_store::adapters::postcard::constants::BUCKET_SIZE_OVERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::COPIED_PAYLOAD_EXCEEDS_OLD_SNAPSHOT;
use crate::outbound::lint_store::adapters::postcard::constants::HEADER_SIZE_OVERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::INLINE_SIZE_OVERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::NO_OLD_CACHE_SNAPSHOT_WAS_READ;
use crate::outbound::lint_store::adapters::postcard::constants::NO_STAGING_SESSION_WAS_READ;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_BUCKET_HAS_TRAILING_BYTES;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_BUCKET_HEADER_IS_TRUNCATED;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_BUCKET_HEADER_OVERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_BUCKET_SPAN_IS_TRUNCATED_OR;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_BUCKET_VARINT_IS_TOO_LONG;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_FILE_NUMBER_OVERFLOWS;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_BELONGS_TO_ANOTHER_STORE;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_COUNT_EXCEEDS_ITS_SECTION;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_DESCRIPTOR_DOES_NOT_MATCH;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_EXCEEDS_ADDRESSABLE_MEMORY;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_HEADER_EXCEEDS_ADDRESSABLE_MEMORY;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_HEADER_EXCEEDS_ITS_ENCODED;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_HEADER_IS_INVERTED;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_HEADER_IS_OUTSIDE_ITS;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_PAYLOAD_EXCEEDS_ADDRESSABLE_MEMORY;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_PAYLOAD_EXCEEDS_ITS_SECTION;
use crate::outbound::lint_store::adapters::postcard::constants::OLD_ROW_PAYLOAD_IS_TRUNCATED_OR;
use crate::outbound::lint_store::adapters::postcard::constants::PAYLOAD_EXCEEDS_POSTCARD_LENGTH_WIDTH;
use crate::outbound::lint_store::adapters::postcard::constants::STORE_OFFSET_OVERFLOWS;
use crate::outbound::lint_store::adapters::postcard::types::FileOffset;
use crate::outbound::lint_store::adapters::postcard::types::Frame;
use crate::outbound::lint_store::adapters::postcard::types::OldFrame;
use crate::outbound::lint_store::adapters::postcard::types::Payload;
use crate::outbound::lint_store::adapters::postcard::types::SectionLength;
use crate::outbound::lint_store::adapters::postcard::types::SectionSpan;
use crate::outbound::lint_store::adapters::postcard::types::Varint;
use crate::outbound::lint_store::adapters::postcard::types::VarintMaximum;
use ryo_values::domain::records::types::HeaderOffset;
use ryo_values::domain::records::types::PayloadLength;
use ryo_values::domain::records::types::PayloadOffset;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use ryo_values::domain::stores::types::OldWords;
use ryo_values::domain::stores::types::StoreSnapshot;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

/// Parse original rows for the unchanged persistent bucket-copy path.
pub(super) fn old_rows(
    file: &StoreFile,
    span: SectionSpan,
) -> std::io::Result<Vec<Frame<'static>>> {
    let held = file.held()?;
    old_rows_in(&mut file.at(FileOffset(0)), span, held)
}

/// The prior frame view remains byte-compatible for writers and their work tests.
pub(super) fn old_rows_in(
    file: &mut (impl Read + Seek),
    span: SectionSpan,
    held: SectionLength,
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
pub(super) fn old_word_rows(file: &StoreFile, span: SectionSpan) -> std::io::Result<Vec<OldFrame>> {
    let held = file.held()?;
    old_word_rows_in(&mut file.at(FileOffset(0)), span, held)
}

pub(super) fn old_word_rows_in(
    file: &mut (impl Read + Seek),
    span: SectionSpan,
    held: SectionLength,
) -> std::io::Result<Vec<OldFrame>> {
    let end = span
        .start
        .0
        .checked_add(span.length.0)
        .filter(|end| *end <= held.0)
        .map(FileOffset)
        .ok_or_else(|| invalid(OLD_BUCKET_SPAN_IS_TRUNCATED_OR))?;
    let _position = file.seek(SeekFrom::Start(span.start.0))?;
    let bounded = BucketReader {
        file,
        start: span.start,
        end,
        at: span.start,
    };
    let mut file = BufReader::with_capacity(BUFFER_BYTES, bounded);
    let mut cursor = span.start;
    let count = integer(
        &mut file,
        &mut cursor,
        end,
        VarintMaximum(u64::try_from(usize::MAX).unwrap_or(u64::MAX)),
    )?
    .0;
    if count > span.length.0 / 2 {
        return Err(invalid(OLD_ROW_COUNT_EXCEEDS_ITS_SECTION));
    }
    let mut rows = Vec::new();
    for _ in 0..count {
        let header = cursor;
        let number = integer(
            &mut file,
            &mut cursor,
            end,
            VarintMaximum(u64::from(u32::MAX)),
        )?
        .0;
        let length = integer(
            &mut file,
            &mut cursor,
            end,
            VarintMaximum(u64::try_from(usize::MAX).unwrap_or(u64::MAX)),
        )?
        .0;
        let after = cursor
            .0
            .checked_add(length)
            .filter(|after| *after <= end.0)
            .map(FileOffset)
            .ok_or_else(|| invalid(OLD_ROW_PAYLOAD_EXCEEDS_ITS_SECTION))?;
        rows.try_reserve(1).map_err(std::io::Error::other)?;
        rows.push(OldFrame {
            file: StoredFileNumber(
                u32::try_from(number).map_err(|_| invalid(OLD_FILE_NUMBER_OVERFLOWS))?,
            ),
            header: HeaderOffset(header.0),
            span: SectionSpan {
                start: cursor,
                length: SectionLength(length),
            },
        });
        seek_buffered(&mut file, PayloadOffset(cursor.0), PayloadOffset(after.0))?;
        cursor = after;
    }
    if cursor != end {
        return Err(invalid(OLD_BUCKET_HAS_TRAILING_BYTES));
    }
    Ok(rows)
}

/// Resolve one row from an already-open immutable snapshot after validating its exact framing.
/// No buffering here can pull another row's payload into the caller's read.
pub(super) fn read_old_in(
    file: &mut (impl Read + Seek),
    held: SectionLength,
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
        .filter(|end| *end <= held.0)
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
        let mut cursor = FileOffset(words.header.0);
        let number = integer(
            &mut fields,
            &mut cursor,
            FileOffset(words.offset.0),
            VarintMaximum(u64::from(u32::MAX)),
        )?
        .0;
        let framed_length = integer(
            &mut fields,
            &mut cursor,
            FileOffset(words.offset.0),
            VarintMaximum(u64::try_from(usize::MAX).unwrap_or(u64::MAX)),
        )?
        .0;
        Ok::<_, std::io::Error>((number, framed_length, cursor))
    })()
    .map_err(|error| {
        into.0.clear();
        error
    })?;
    let (number, framed_length, cursor) = framing;
    if number != u64::from(words.file.0)
        || framed_length != words.length.0
        || cursor.0 != words.offset.0
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
pub(super) fn read_old_file_in(
    file: &StoreFile,
    held: SectionLength,
    snapshot: StoreSnapshot,
    bucket: SectionSpan,
    words: &OldWords,
    into: &mut PackedWords,
) -> std::io::Result<()> {
    read_old_in(
        &mut file.at(FileOffset(0)),
        held,
        snapshot,
        bucket,
        words,
        into,
    )
}

/// Postcard unsigned varints are bounded by the encoded type's width, including a checked last digit.
fn integer(
    file: &mut impl Read,
    cursor: &mut FileOffset,
    end: FileOffset,
    maximum: VarintMaximum,
) -> std::io::Result<Varint> {
    let digits = ((64 - maximum.0.leading_zeros()).max(1)).div_ceil(7);
    let mut value = 0_u64;
    for digit in 0..digits {
        if cursor.0 >= end.0 {
            return Err(invalid(OLD_BUCKET_HEADER_IS_TRUNCATED));
        }
        let mut byte = [0_u8; 1];
        file.read_exact(&mut byte)?;
        cursor.0 += 1;
        let payload = u64::from(byte[0] & 127);
        let shift = digit * 7;
        if payload > maximum.0 >> shift {
            return Err(invalid(OLD_BUCKET_HEADER_OVERFLOWS));
        }
        value |= payload << shift;
        if byte[0] & 128 == 0 {
            return Ok(Varint(value));
        }
    }
    Err(invalid(OLD_BUCKET_VARINT_IS_TOO_LONG))
}

/// Serialize numbers with the SAME codec used by the prior whole-sequence serializer.
fn header<T: serde::Serialize>(into: &mut impl Write, value: &T) -> std::io::Result<SectionLength> {
    let mut buffer = [0_u8; 16];
    let bytes = postcard::to_slice(value, &mut buffer).map_err(std::io::Error::other)?;
    into.write_all(bytes)?;
    u64::try_from(bytes.len())
        .map(SectionLength)
        .map_err(|_| invalid(HEADER_SIZE_OVERFLOWS))
}

/// Write one existing-format bucket, streaming every selected payload without a rebuilt Vec.
pub(super) fn write_bucket(
    into: &mut impl Write,
    at: &mut FileOffset,
    rows: &[Frame<'_>],
    old: Option<&StoreFile>,
    mut staged: Option<&mut StagingConnection>,
) -> std::io::Result<SectionSpan> {
    // Snapshot size is validated once per bucket, not once per tiny row.
    let old_length = old.map(|file| file.held().map(|held| held.0)).transpose()?;
    let mut old_position = 0;
    let mut old = old.map(|file| BufReader::with_capacity(BUFFER_BYTES, file.at(FileOffset(0))));
    if let Some(spool) = staged.as_deref_mut() {
        spool.flush()?;
    }
    let start = *at;
    let mut length = header(into, &rows.len())?.0;
    for row in rows {
        let size = match row.payload {
            Payload::Inline(words) => {
                u64::try_from(words.0.len()).map_err(|_| invalid(INLINE_SIZE_OVERFLOWS))?
            }
            Payload::Staged(span) => span.length.0,
            Payload::Old(span) => span.length.0,
        };
        let addressable =
            usize::try_from(size).map_err(|_| invalid(PAYLOAD_EXCEEDS_POSTCARD_LENGTH_WIDTH))?;
        let file_header = header(into, &row.file.0)?.0;
        let size_header = header(into, &addressable)?.0;
        length = length
            .checked_add(file_header)
            .and_then(|length| length.checked_add(size_header))
            .ok_or_else(|| invalid(BUCKET_SIZE_OVERFLOWS))?;
        match row.payload {
            Payload::Inline(words) => into.write_all(&words.0)?,
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
                    seek_buffered(
                        reader,
                        PayloadOffset(old_position),
                        PayloadOffset(span.start.0),
                    )?;
                }
                copy_buffered(reader, into, PayloadLength(span.length.0))?;
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
