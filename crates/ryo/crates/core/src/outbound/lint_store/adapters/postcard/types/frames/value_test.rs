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
use ryo_values::domain::stores::types::{FileAgain, StagedRow, StoredFileNumber, StoredWords};
use std::io::Write as _;

/// Distinguishable rows include duplicate file identities and arbitrary packed bytes.
fn row(file: u32, length: usize) -> StoredWords {
    StoredWords {
        file: StoredFileNumber(file),
        bucket: Bucket(3),
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
            ordinal: 1,
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
        ordinal: 1,
        words,
    };
    assert!(fresh(&[], &[row]).is_err());
    assert!(
        fresh(
            &[],
            &[
                StagedRow { ordinal: 0, ..row },
                StagedRow { ordinal: 0, ..row }
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
    assert!(output.largest < usize::try_from(span.length).expect("size"));
    assert!(output.bytes >= usize::try_from(span.length).expect("size"));
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
