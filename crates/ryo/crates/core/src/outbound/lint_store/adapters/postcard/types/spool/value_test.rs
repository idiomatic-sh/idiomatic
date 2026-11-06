//! Session lifetime, bounded selected replay and every staging IO failure are explicit.

use super::value::{StagingSpool, append, read, validate};
use ryo_values::domain::records::types::{StagedWords, StagingSession};
use ryo_values::domain::stored_facts::types::PackedWords;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

#[derive(Default)]
struct FaultFile {
    bytes: Cursor<Vec<u8>>,
    write_fails: bool,
    read_fails: bool,
    flush_fails: bool,
}
impl Write for FaultFile {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.write_fails {
            return Err(std::io::Error::other("write failure"));
        }
        self.bytes.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if self.flush_fails {
            return Err(std::io::Error::other("flush failure"));
        }
        Ok(())
    }
}
impl Read for FaultFile {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        if self.read_fails {
            return Err(std::io::Error::other("read failure"));
        }
        self.bytes.read(bytes)
    }
}
impl Seek for FaultFile {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.bytes.seek(position)
    }
}

#[test]
fn spool_lifetime_and_foreign_overflow_truncation_checks_are_not_empty_fallbacks() {
    let mut spool = StagingSpool::new(StagingSession(1)).expect("temporary spool");
    let path = spool.file.path().to_owned();
    assert!(path.exists());
    let first = spool.append(PackedWords(vec![1, 2, 3])).expect("append");
    let second = spool
        .append(PackedWords(vec![9; 10000]))
        .expect("second append");
    let mut loaded = PackedWords::default();
    spool
        .read(&first, &mut loaded)
        .expect("selected first only");
    assert_eq!(loaded.0, [1, 2, 3]);
    assert!(
        loaded.0.capacity() < usize::try_from(second.length.0).expect("size"),
        "never read every staged bucket at once"
    );
    assert!(
        spool
            .read(
                &StagedWords {
                    session: StagingSession(2),
                    ..first
                },
                &mut loaded
            )
            .is_err()
    );
    assert!(
        validate(
            spool.session,
            spool.length,
            &StagedWords {
                offset: ryo_values::domain::records::types::PayloadOffset(u64::MAX),
                length: ryo_values::domain::records::types::PayloadLength(1),
                ..first
            }
        )
        .is_err()
    );
    assert!(
        validate(
            spool.session,
            spool.length,
            &StagedWords {
                offset: ryo_values::domain::records::types::PayloadOffset(0),
                length: ryo_values::domain::records::types::PayloadLength(u64::MAX),
                ..first
            }
        )
        .is_err()
    );
    spool
        .file
        .as_file_mut()
        .set_len(1)
        .expect("truncate injected input");
    let _empty = spool
        .append(PackedWords::default())
        .expect("append mode discards prior read-ahead without extending the truncated file");
    assert!(spool.read(&second, &mut loaded).is_err());
    drop(spool);
    assert!(
        !path.exists(),
        "RAII closes and deletes on success or failure"
    );
}

#[test]
fn failed_creation_write_flush_and_read_do_not_commit_a_descriptor() {
    assert!(
        StagingSpool::create(StagingSession(1), || Err(std::io::Error::other(
            "create failure"
        )))
        .is_err()
    );
    for (write_fails, flush_fails) in [(true, false), (false, true)] {
        let mut file = FaultFile {
            write_fails,
            flush_fails,
            ..FaultFile::default()
        };
        let mut held = 0;
        assert!(
            append(
                &mut file,
                StagingSession(1),
                &mut held,
                &PackedWords(vec![1, 2])
            )
            .is_err()
        );
        assert_eq!(held, 0);
    }
    let mut file = FaultFile::default();
    let mut held = 0;
    let span = append(
        &mut file,
        StagingSession(1),
        &mut held,
        &PackedWords(vec![1, 2]),
    )
    .expect("valid control");
    let mut out = PackedWords::default();
    file.read_fails = true;
    assert!(read(&mut file, StagingSession(1), held, &span, &mut out).is_err());
    file.read_fails = false;
    file.flush_fails = true;
    assert!(read(&mut file, StagingSession(1), held, &span, &mut out).is_err());
    let mut impossible = u64::MAX;
    assert!(
        append(
            &mut file,
            StagingSession(1),
            &mut impossible,
            &PackedWords(vec![0])
        )
        .is_err()
    );
    assert_eq!(impossible, u64::MAX);
}

#[test]
fn contiguous_tiny_payloads_use_chunk_io_instead_of_row_io() {
    use super::value::{BUFFER_BYTES, copy_buffered};
    use std::cell::Cell;
    use std::io::{BufReader, BufWriter};
    use std::rc::Rc;
    #[derive(Default)]
    struct Counts {
        reads: Cell<usize>,
        writes: Cell<usize>,
        seeks: Cell<usize>,
    }
    struct Counted {
        inner: Cursor<Vec<u8>>,
        counts: Rc<Counts>,
    }
    impl Read for Counted {
        fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
            self.counts.reads.set(self.counts.reads.get() + 1);
            self.inner.read(bytes)
        }
    }
    impl Write for Counted {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.counts.writes.set(self.counts.writes.get() + 1);
            self.inner.write(bytes)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Seek for Counted {
        fn seek(&mut self, at: SeekFrom) -> std::io::Result<u64> {
            self.counts.seeks.set(self.counts.seeks.get() + 1);
            self.inner.seek(at)
        }
    }
    let rows = 10_000;
    let width = 13;
    let payload = vec![7; rows * width];
    let counts = Rc::new(Counts::default());
    let mut read = BufReader::with_capacity(
        BUFFER_BYTES,
        Counted {
            inner: Cursor::new(payload.clone()),
            counts: Rc::clone(&counts),
        },
    );
    let mut write = BufWriter::with_capacity(
        BUFFER_BYTES,
        Counted {
            inner: Cursor::new(Vec::new()),
            counts: Rc::clone(&counts),
        },
    );
    for _ in 0..rows {
        copy_buffered(&mut read, &mut write, u64::try_from(width).expect("width"))
            .expect("one span");
    }
    write.flush().expect("publication barrier");
    assert_eq!(write.get_ref().inner.get_ref(), &payload);
    assert!(counts.reads.get() <= payload.len().div_ceil(BUFFER_BYTES) + 1);
    assert!(counts.writes.get() <= payload.len().div_ceil(BUFFER_BYTES) + 1);
    assert_eq!(counts.seeks.get(), 0);
    // Buffer acceptance is not publication: a later flush refusal remains an error.
    let mut failing = BufWriter::with_capacity(
        BUFFER_BYTES,
        FaultFile {
            write_fails: true,
            ..FaultFile::default()
        },
    );
    failing
        .write_all(b"accepted only into memory")
        .expect("buffered");
    assert!(failing.flush().is_err());
}
