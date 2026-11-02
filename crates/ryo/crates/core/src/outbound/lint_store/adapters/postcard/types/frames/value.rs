//! Compact row-span metadata and bounded copies preserve the existing postcard sequence format.

use crate::outbound::lint_store::adapters::postcard::types::spool::value::{
    BUFFER_BYTES, copy_buffered, seek_buffered,
};
use crate::outbound::lint_store::adapters::postcard::types::spool::value::{StagingSpool, invalid};
use crate::outbound::lint_store::adapters::postcard::types::{
    FileOffset, SectionLength, SectionSpan,
};
use ryo_values::domain::records::types::StagedWords;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{StagedRow, StoredFileNumber, StoredWords};
use std::collections::BTreeMap;
use std::io::{BufReader, Seek as _, SeekFrom, Write};

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

/// Restore one fresh sequence from inline entries and staged ordinal slots; no payload is copied.
pub(in crate::outbound::lint_store::adapters::postcard) fn fresh<'a>(
    inline: &'a [StoredWords],
    staged: &[StagedRow],
) -> std::io::Result<BTreeMap<Bucket, Vec<Frame<'a>>>> {
    let count = inline
        .len()
        .checked_add(staged.len())
        .ok_or_else(|| invalid("fresh row count overflows"))?;
    let mut spans: Vec<_> = staged.iter().collect();
    spans.sort_by_key(|row| row.ordinal);
    if spans.iter().any(|row| {
        usize::try_from(row.ordinal)
            .ok()
            .is_none_or(|ordinal| ordinal >= count)
    }) || spans
        .windows(2)
        .any(|pair| pair[0].ordinal == pair[1].ordinal)
    {
        return Err(invalid("invalid fresh staged row order"));
    }
    let mut spans = spans.into_iter().peekable();
    let mut inline = inline.iter();
    let mut buckets: BTreeMap<Bucket, Vec<Frame<'a>>> = BTreeMap::new();
    for ordinal in
        0..u64::try_from(count).map_err(|_| invalid("fresh row count exceeds staging width"))?
    {
        if spans.peek().is_some_and(|row| row.ordinal == ordinal) {
            let row = spans.next().ok_or_else(|| invalid("missing staged row"))?;
            buckets.entry(row.bucket).or_default().push(Frame {
                file: row.file,
                payload: Payload::Staged(row.words),
            });
        } else {
            let row = inline
                .next()
                .ok_or_else(|| invalid("missing inline fresh row"))?;
            buckets.entry(row.bucket).or_default().push(Frame {
                file: row.file,
                payload: Payload::Inline(&row.words.0),
            });
        }
    }
    Ok(buckets)
}

/// Parse only row headers and byte spans, bounded by the original section; never read its packed payload.
pub(in crate::outbound::lint_store::adapters::postcard) fn old_rows(
    file: &mut std::fs::File,
    span: SectionSpan,
) -> std::io::Result<Vec<Frame<'static>>> {
    let held = file.metadata()?.len();
    old_rows_in(file, span, held)
}

/// The exact header scanner is generic so tests count physical IO independent of filesystem caching.
pub(in crate::outbound::lint_store::adapters::postcard) fn old_rows_in(
    file: &mut (impl std::io::Read + std::io::Seek),
    span: SectionSpan,
    held: u64,
) -> std::io::Result<Vec<Frame<'static>>> {
    let end = span
        .start
        .0
        .checked_add(span.length.0)
        .filter(|end| *end <= held)
        .ok_or_else(|| invalid("old bucket span is truncated or overflows"))?;
    let _position = file.seek(SeekFrom::Start(span.start.0))?;
    let mut file = BufReader::with_capacity(BUFFER_BYTES, file);
    let mut cursor = span.start.0;
    let count = integer(
        &mut file,
        &mut cursor,
        end,
        u64::try_from(usize::MAX).unwrap_or(u64::MAX),
    )?;
    if count > span.length.0 / 2 {
        return Err(invalid("old row count exceeds its section"));
    }
    let mut rows = Vec::new();
    for _ in 0..count {
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
            .ok_or_else(|| invalid("old row payload exceeds its section"))?;
        rows.try_reserve(1).map_err(std::io::Error::other)?;
        rows.push(Frame {
            file: StoredFileNumber(
                u32::try_from(number).map_err(|_| invalid("old file number overflows"))?,
            ),
            payload: Payload::Old(SectionSpan {
                start: FileOffset(cursor),
                length: SectionLength(length),
            }),
        });
        seek_buffered(&mut file, cursor, after)?;
        cursor = after;
    }
    if cursor != end {
        return Err(invalid("old bucket has trailing bytes"));
    }
    Ok(rows)
}

/// Postcard unsigned varints are bounded by the encoded type's width, including a checked last digit.
fn integer(
    file: &mut impl std::io::Read,
    cursor: &mut u64,
    end: u64,
    maximum: u64,
) -> std::io::Result<u64> {
    let digits = ((64 - maximum.leading_zeros()).max(1)).div_ceil(7);
    let mut value = 0_u64;
    for digit in 0..digits {
        if *cursor >= end {
            return Err(invalid("old bucket header is truncated"));
        }
        let mut byte = [0_u8; 1];
        file.read_exact(&mut byte)?;
        *cursor += 1;
        let payload = u64::from(byte[0] & 127);
        let shift = digit * 7;
        if payload > maximum >> shift {
            return Err(invalid("old bucket header overflows"));
        }
        value |= payload << shift;
        if byte[0] & 128 == 0 {
            return Ok(value);
        }
    }
    Err(invalid("old bucket varint is too long"))
}

/// Serialize numbers with the SAME codec used by the prior whole-sequence serializer.
fn header<T: serde::Serialize>(into: &mut impl Write, value: &T) -> std::io::Result<u64> {
    let mut buffer = [0_u8; 16];
    let bytes = postcard::to_slice(value, &mut buffer).map_err(std::io::Error::other)?;
    into.write_all(bytes)?;
    u64::try_from(bytes.len()).map_err(|_| invalid("header size overflows"))
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
                u64::try_from(bytes.len()).map_err(|_| invalid("inline size overflows"))?
            }
            Payload::Staged(span) => span.length,
            Payload::Old(span) => span.length.0,
        };
        let addressable =
            usize::try_from(size).map_err(|_| invalid("payload exceeds postcard length width"))?;
        let file_header = header(into, &row.file.0)?;
        let size_header = header(into, &addressable)?;
        length = length
            .checked_add(file_header)
            .and_then(|length| length.checked_add(size_header))
            .ok_or_else(|| invalid("bucket size overflows"))?;
        match row.payload {
            Payload::Inline(bytes) => into.write_all(bytes)?,
            Payload::Staged(span) => staged
                .as_deref_mut()
                .ok_or_else(|| invalid("no staging session was read"))?
                .copy(&span, into)?,
            Payload::Old(span) => {
                let end = span
                    .start
                    .0
                    .checked_add(span.length.0)
                    .filter(|end| old_length.is_some_and(|held| *end <= held))
                    .ok_or_else(|| invalid("copied payload exceeds old snapshot"))?;
                let reader = old
                    .as_mut()
                    .ok_or_else(|| invalid("no old cache snapshot was read"))?;
                if old_position != span.start.0 {
                    seek_buffered(reader, old_position, span.start.0)?;
                }
                copy_buffered(reader, into, span.length.0)?;
                old_position = end;
            }
        }
        length = length
            .checked_add(size)
            .ok_or_else(|| invalid("bucket size overflows"))?;
    }
    at.0 =
        at.0.checked_add(length)
            .ok_or_else(|| invalid("store offset overflows"))?;
    Ok(SectionSpan {
        start,
        length: SectionLength(length),
    })
}
