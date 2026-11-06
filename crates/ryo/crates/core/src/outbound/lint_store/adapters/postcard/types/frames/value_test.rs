//! Streaming preserves postcard bytes and stable row order without owning old payloads.

use super::value::{Payload, fresh, old_rows, write_bucket};
use crate::outbound::lint_store::adapters::postcard::schemas::{
    bucket_bytes, rebuilt_bucket_bytes,
};
use crate::outbound::lint_store::adapters::postcard::types::spool::value::StagingSpool;
use crate::outbound::lint_store::adapters::postcard::types::{
    FileOffset, SectionLength, SectionSpan,
};
use ryo_values::domain::records::types::StagingSession;
use ryo_values::domain::stored_facts::types::{Bucket, PackedWords};
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use ryo_values::domain::stores::types::{FileAgain, StagedRow, StoredWords};
use std::io::Write as _;

/// Distinguishable rows include duplicate file identities and arbitrary packed bytes.
fn row(file: u32, length: usize) -> StoredWords {
    StoredWords {
        file: StoredFileNumber(file),
        bucket: Bucket(3),
        proof: crate::outbound::lint_store::fixtures::proof(),
        words: PackedWords(
            (0..length)
                .map(|at| u8::try_from(at % 256).expect("byte"))
                .collect(),
        ),
    }
}

#[test]
fn inline_and_staged_mixed_rows_match_legacy_bytes_at_varint_boundaries() {
    for length in [0, 1, 127, 128, 16383, 16384, 100_000] {
        let rows = [row(128, length), row(0, 0), row(128, 9)];
        let expected = bucket_bytes(&rows.iter().collect::<Vec<_>>()).expect("legacy bytes");
        let mut spool = StagingSpool::new(StagingSession(1)).expect("spool");
        let span = spool.append(rows[1].words.clone()).expect("stage");
        let staged = [StagedRow {
            file: rows[1].file,
            bucket: rows[1].bucket,
            ordinal: ryo_values::domain::records::types::StagedOrdinal(1),
            words: span,
        }];
        let inline = [rows[0].clone(), rows[2].clone()];
        let groups = fresh(&inline, &staged).expect("mixed order");
        let mut bytes = Vec::new();
        let mut at = FileOffset(0);
        let _section = write_bucket(
            &mut bytes,
            &mut at,
            &groups[&Bucket(3)],
            None,
            Some(&mut spool),
        )
        .expect("stream");
        assert_eq!(bytes, expected);
        assert_eq!(at.0, u64::try_from(expected.len()).expect("size"));
    }
    let mut bytes = Vec::new();
    let mut at = FileOffset(0);
    let _empty_section = write_bucket(&mut bytes, &mut at, &[], None, None).expect("empty bucket");
    assert_eq!(bytes, bucket_bytes(&[]).expect("empty legacy"));
}

#[test]
fn old_spans_and_fresh_rows_preserve_selected_file_remapping_and_tie_order() {
    let old = [row(0, 128), row(1, 16384), row(1, 7), row(2, 99)];
    let old_bytes = bucket_bytes(&old.iter().collect::<Vec<_>>()).expect("legacy");
    let mut file = tempfile::tempfile().expect("old snapshot");
    file.write_all(&old_bytes).expect("write");
    let span = SectionSpan {
        start: FileOffset(0),
        length: SectionLength(u64::try_from(old_bytes.len()).expect("size")),
    };
    let parsed = old_rows(&mut file, span).expect("headers only");
    assert!(
        parsed
            .iter()
            .all(|row| matches!(row.payload, Payload::Old(_)))
    );
    let again = [FileAgain {
        was: StoredFileNumber(1),
        now: StoredFileNumber(0),
    }];
    let fresh_rows = [row(0, 3), row(2, 4)];
    let mut groups = fresh(&fresh_rows, &[]).expect("fresh");
    let rows = groups.get_mut(&Bucket(3)).expect("bucket");
    rows.extend(parsed.into_iter().filter_map(|mut row| {
        let mapping = again.iter().find(|entry| entry.was == row.file)?;
        row.file = mapping.now;
        Some(row)
    }));
    rows.sort_by_key(|row| row.file);
    let mut actual = Vec::new();
    let mut at = FileOffset(0);
    let _copied_section =
        write_bucket(&mut actual, &mut at, rows, Some(&mut file), None).expect("bounded copy");
    assert_eq!(
        Some(actual),
        rebuilt_bucket_bytes(
            &fresh_rows.iter().collect::<Vec<_>>(),
            Some(&old_bytes),
            &again
        )
        .expect("legacy rebuilt")
    );
}

#[test]
fn corrupt_headers_order_and_truncated_payloads_are_refused() {
    for bytes in [vec![], vec![1], vec![1, 0, 9, 1], vec![0, 0], vec![255; 12]] {
        let mut file = tempfile::tempfile().expect("file");
        file.write_all(&bytes).expect("write");
        assert!(
            old_rows(
                &mut file,
                SectionSpan {
                    start: FileOffset(0),
                    length: SectionLength(u64::try_from(bytes.len()).expect("size"))
                }
            )
            .is_err()
        );
    }
    let mut spool = StagingSpool::new(StagingSession(2)).expect("spool");
    let words = spool.append(PackedWords(vec![1])).expect("stage");
    let row = StagedRow {
        file: StoredFileNumber(0),
        bucket: Bucket(3),
        ordinal: ryo_values::domain::records::types::StagedOrdinal(1),
        words,
    };
    assert!(fresh(&[], &[row]).is_err());
    assert!(
        fresh(
            &[],
            &[
                StagedRow {
                    ordinal: ryo_values::domain::records::types::StagedOrdinal(0),
                    ..row
                },
                StagedRow {
                    ordinal: ryo_values::domain::records::types::StagedOrdinal(0),
                    ..row
                }
            ]
        )
        .is_err()
    );
    let frames = [super::value::Frame {
        file: StoredFileNumber(0),
        payload: Payload::Staged(words),
    }];
    let mut at = FileOffset(u64::MAX);
    assert!(write_bucket(&mut Vec::new(), &mut at, &frames, None, Some(&mut spool)).is_err());
    assert!(
        write_bucket(
            &mut std::io::sink(),
            &mut FileOffset(0),
            &frames,
            None,
            None
        )
        .is_err()
    );
}

#[test]
fn streamed_payload_copies_have_bounded_write_chunks_and_propagate_write_failure() {
    struct ObservedWriter {
        largest: usize,
        bytes: usize,
        fail_after: usize,
    }
    impl std::io::Write for ObservedWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.largest = self.largest.max(bytes.len());
            if self.bytes >= self.fail_after {
                return Err(std::io::Error::other("injected final write failure"));
            }
            self.bytes += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut spool = StagingSpool::new(StagingSession(4)).expect("spool");
    let span = spool
        .append(PackedWords(vec![5; 1024 * 1024]))
        .expect("large frontier");
    let rows = [super::value::Frame {
        file: StoredFileNumber(0),
        payload: Payload::Staged(span),
    }];
    let mut output = ObservedWriter {
        largest: 0,
        bytes: 0,
        fail_after: usize::MAX,
    };
    let _streamed_section = write_bucket(
        &mut output,
        &mut FileOffset(0),
        &rows,
        None,
        Some(&mut spool),
    )
    .expect("bounded copy");
    assert!(output.largest < usize::try_from(span.length.0).expect("size"));
    assert!(output.bytes >= usize::try_from(span.length.0).expect("size"));
    let mut failing = ObservedWriter {
        largest: 0,
        bytes: 0,
        fail_after: 16,
    };
    let mut position = FileOffset(9);
    assert!(write_bucket(&mut failing, &mut position, &rows, None, Some(&mut spool)).is_err());
    assert_eq!(
        position,
        FileOffset(9),
        "failed bucket never publishes a completed span"
    );
}

#[test]
fn scanning_tiny_old_rows_reads_and_seeks_by_chunks() {
    use super::value::old_rows_in;
    use crate::outbound::lint_store::adapters::postcard::types::spool::value::BUFFER_BYTES;
    use std::io::{Cursor, Read, Seek, SeekFrom};
    struct Counted {
        inner: Cursor<Vec<u8>>,
        reads: usize,
        seeks: usize,
    }
    impl Read for Counted {
        fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
            self.reads += 1;
            self.inner.read(into)
        }
    }
    impl Seek for Counted {
        fn seek(&mut self, at: SeekFrom) -> std::io::Result<u64> {
            self.seeks += 1;
            self.inner.seek(at)
        }
    }
    let rows = vec![row(0, 13); 10_000];
    let bytes = bucket_bytes(&rows.iter().collect::<Vec<_>>()).expect("existing wire");
    let length = u64::try_from(bytes.len()).expect("size");
    let mut file = Counted {
        inner: Cursor::new(bytes),
        reads: 0,
        seeks: 0,
    };
    let spans = old_rows_in(
        &mut file,
        SectionSpan {
            start: FileOffset(0),
            length: SectionLength(length),
        },
        length,
    )
    .expect("scan exact production path");
    assert_eq!(spans.len(), rows.len());
    let chunks = usize::try_from(length)
        .expect("size")
        .div_ceil(BUFFER_BYTES);
    assert!(
        file.reads <= chunks + 1,
        "reads are buffered, not one per header byte"
    );
    assert!(
        file.seeks <= chunks + 2,
        "small payload skips stay inside the buffer"
    );
}

/// A virtual huge bucket exposes actual read volume without allocating its payload set.
#[test]
fn old_descriptor_enumeration_skips_huge_payloads_and_resolution_reads_one() {
    use super::value::{old_word_rows_in, read_old_in};
    use crate::outbound::lint_store::adapters::postcard::types::spool::value::BUFFER_BYTES;
    use ryo_values::domain::stores::types::{OldWords, StoreSnapshot};
    use std::io::{Read, Seek, SeekFrom};
    struct Sparse {
        headers: Vec<(u64, Vec<u8>)>,
        length: u64,
        at: u64,
        bytes: usize,
    }
    impl Read for Sparse {
        fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
            let length = into
                .len()
                .min(usize::try_from(self.length - self.at).unwrap());
            into[..length].fill(0x5a);
            for (start, header) in &self.headers {
                let begin = self.at.max(*start);
                let end = (self.at + length as u64).min(*start + header.len() as u64);
                if begin < end {
                    into[(begin - self.at) as usize..(end - self.at) as usize]
                        .copy_from_slice(&header[(begin - start) as usize..(end - start) as usize]);
                }
            }
            self.at += length as u64;
            self.bytes += length;
            Ok(length)
        }
    }
    impl Seek for Sparse {
        fn seek(&mut self, seek: SeekFrom) -> std::io::Result<u64> {
            let at = match seek {
                SeekFrom::Start(at) => i128::from(at),
                SeekFrom::Current(by) => i128::from(self.at) + i128::from(by),
                SeekFrom::End(by) => i128::from(self.length) + i128::from(by),
            };
            self.at = u64::try_from(at).map_err(std::io::Error::other)?;
            Ok(self.at)
        }
    }
    let count = postcard::to_allocvec(&3_usize).unwrap();
    let mut at = count.len() as u64;
    let mut headers = vec![(0, count)];
    let payload = 2 * 1024 * 1024_usize;
    for file in [2_u32, 0, 2] {
        let mut header = postcard::to_allocvec(&file).unwrap();
        header.extend(postcard::to_allocvec(&payload).unwrap());
        let header_length = header.len() as u64;
        headers.push((at, header));
        at += header_length + payload as u64;
    }
    let mut file = Sparse {
        headers,
        length: at,
        at: 0,
        bytes: 0,
    };
    let bucket = SectionSpan {
        start: FileOffset(0),
        length: SectionLength(at),
    };
    let rows = old_word_rows_in(&mut file, bucket, at).expect("header scan");
    assert_eq!(
        rows.iter().map(|row| row.file.0).collect::<Vec<_>>(),
        [2, 0, 2]
    );
    assert!(
        file.bytes <= 3 * BUFFER_BYTES,
        "only bounded header read-ahead, never six MiB payload ownership"
    );
    let descriptor = OldWords {
        snapshot: StoreSnapshot(91),
        bucket: Bucket(3),
        file: rows[1].file,
        header: ryo_values::domain::records::types::HeaderOffset(rows[1].header),
        offset: ryo_values::domain::records::types::PayloadOffset(rows[1].span.start.0),
        length: ryo_values::domain::records::types::PayloadLength(rows[1].span.length.0),
    };
    file.bytes = 0;
    let mut loaded = PackedWords::default();
    read_old_in(
        &mut file,
        at,
        StoreSnapshot(91),
        bucket,
        &descriptor,
        &mut loaded,
    )
    .expect("one payload");
    assert_eq!(loaded.0.len(), payload);
    assert!(loaded.0.iter().all(|byte| *byte == 0x5a));
    assert_eq!(
        file.bytes as u64,
        descriptor.length.0 + descriptor.offset.0 - descriptor.header.0
    );
    // A tiny requested bucket must not read ahead into an adjacent unrequested section.
    let mut tiny = Sparse {
        headers: vec![(0, vec![1, 0, 0])],
        length: 1024 * 1024,
        at: 0,
        bytes: 0,
    };
    let tiny_rows = old_word_rows_in(
        &mut tiny,
        SectionSpan {
            start: FileOffset(0),
            length: SectionLength(3),
        },
        1024 * 1024,
    )
    .expect("one empty row");
    assert_eq!(tiny_rows.len(), 1);
    assert_eq!(
        tiny.bytes, 3,
        "no bytes from the unrequested neighboring bucket"
    );
    assert!(
        read_old_in(
            &mut file,
            at - 1,
            StoreSnapshot(91),
            bucket,
            &descriptor,
            &mut loaded
        )
        .is_err()
    );
    for bad in [
        OldWords {
            snapshot: StoreSnapshot(92),
            ..descriptor
        },
        OldWords {
            length: ryo_values::domain::records::types::PayloadLength(u64::MAX),
            ..descriptor
        },
        OldWords {
            header: ryo_values::domain::records::types::HeaderOffset(u64::MAX),
            ..descriptor
        },
        OldWords {
            offset: ryo_values::domain::records::types::PayloadOffset(descriptor.offset.0 + 1),
            ..descriptor
        },
        OldWords {
            file: StoredFileNumber(99),
            ..descriptor
        },
    ] {
        assert!(read_old_in(&mut file, at, StoreSnapshot(91), bucket, &bad, &mut loaded).is_err());
    }
}

#[test]
fn old_payloads_and_headers_share_one_exact_read_without_neighbor_prefetch() {
    use super::value::{old_word_rows_in, read_old_in};
    use ryo_values::domain::stores::types::{OldWords, StoreSnapshot};
    use std::io::{Read, Seek, SeekFrom};

    struct Counted {
        bytes: std::io::Cursor<Vec<u8>>,
        reads: usize,
        transferred: usize,
    }
    impl Read for Counted {
        fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
            self.reads += 1;
            let count = self.bytes.read(into)?;
            self.transferred += count;
            Ok(count)
        }
    }
    impl Seek for Counted {
        fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
            self.bytes.seek(from)
        }
    }

    for file_number in [0, 127, 128, u32::MAX] {
        for length in [0, 1, 127, 128, 16_384] {
            let selected = row(file_number, length);
            let neighbor = row(7, 23);
            let bytes = bucket_bytes(&[&selected, &neighbor]).expect("two rows");
            let held = u64::try_from(bytes.len()).expect("length");
            let bucket = SectionSpan {
                start: FileOffset(0),
                length: SectionLength(held),
            };
            let mut file = Counted {
                bytes: std::io::Cursor::new(bytes),
                reads: 0,
                transferred: 0,
            };
            let headers = old_word_rows_in(&mut file, bucket, held).expect("framing");
            let first = headers.first().expect("selected row");
            let descriptor = OldWords {
                snapshot: StoreSnapshot(81),
                bucket: Bucket(3),
                file: first.file,
                header: ryo_values::domain::records::types::HeaderOffset(first.header),
                offset: ryo_values::domain::records::types::PayloadOffset(first.span.start.0),
                length: ryo_values::domain::records::types::PayloadLength(first.span.length.0),
            };
            file.reads = 0;
            file.transferred = 0;
            let mut loaded = PackedWords::default();
            read_old_in(
                &mut file,
                held,
                StoreSnapshot(81),
                bucket,
                &descriptor,
                &mut loaded,
            )
            .expect("selected payload");
            assert_eq!(loaded, selected.words);
            assert_eq!(file.reads, 1);
            assert_eq!(
                u64::try_from(file.transferred).expect("count"),
                descriptor.offset.0 - descriptor.header.0 + descriptor.length.0,
                "only this exact header and payload are read"
            );
            let malformed = OldWords {
                header: ryo_values::domain::records::types::HeaderOffset(bucket.start.0 + 1),
                offset: ryo_values::domain::records::types::PayloadOffset(bucket.start.0 + 17),
                length: ryo_values::domain::records::types::PayloadLength(0),
                ..descriptor
            };
            file.reads = 0;
            assert!(
                read_old_in(
                    &mut file,
                    held,
                    StoreSnapshot(81),
                    bucket,
                    &malformed,
                    &mut loaded
                )
                .is_err()
            );
            assert_eq!(file.reads, 0, "oversized headers are refused before IO");
            let header = usize::try_from(descriptor.header.0).expect("header offset");
            file.bytes.get_mut()[header] ^= 1;
            assert!(
                read_old_in(
                    &mut file,
                    held,
                    StoreSnapshot(81),
                    bucket,
                    &descriptor,
                    &mut loaded
                )
                .is_err(),
                "changed framing is refused after snapshot enumeration"
            );
            assert!(loaded.0.is_empty(), "rejected payload is not returned");
        }
    }
}

#[cfg(unix)]
#[test]
fn positional_old_payload_read_preserves_cursor_and_refuses_late_truncation() {
    use super::value::{old_word_rows_in, read_old_file_in};
    use ryo_values::domain::stores::types::{OldWords, StoreSnapshot};
    use std::io::{Seek as _, SeekFrom};

    let selected = row(128, 16_384);
    let neighbor = row(7, 23);
    let bytes = bucket_bytes(&[&selected, &neighbor]).expect("two rows");
    let held = u64::try_from(bytes.len()).expect("length");
    let bucket = SectionSpan {
        start: FileOffset(0),
        length: SectionLength(held),
    };
    let mut file = tempfile::tempfile().expect("snapshot");
    file.write_all(&bytes).expect("write rows");
    let headers = old_word_rows_in(&mut file, bucket, held).expect("framing");
    let first = headers.first().expect("selected row");
    let descriptor = OldWords {
        snapshot: StoreSnapshot(81),
        bucket: Bucket(3),
        file: first.file,
        header: ryo_values::domain::records::types::HeaderOffset(first.header),
        offset: ryo_values::domain::records::types::PayloadOffset(first.span.start.0),
        length: ryo_values::domain::records::types::PayloadLength(first.span.length.0),
    };
    let cursor = file.seek(SeekFrom::End(0)).expect("cursor");
    let mut loaded = PackedWords::default();
    read_old_file_in(
        &file,
        held,
        StoreSnapshot(81),
        bucket,
        &descriptor,
        &mut loaded,
    )
    .expect("positional payload");
    assert_eq!(loaded, selected.words);
    assert_eq!(file.seek(SeekFrom::Current(0)).expect("cursor"), cursor);

    file.set_len(descriptor.offset.0 + descriptor.length.0 - 1)
        .expect("truncate selected payload");
    assert!(
        read_old_file_in(
            &file,
            held,
            StoreSnapshot(81),
            bucket,
            &descriptor,
            &mut loaded
        )
        .is_err()
    );
    assert!(loaded.0.is_empty());
}
