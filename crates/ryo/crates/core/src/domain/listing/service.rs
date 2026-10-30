//! The `listing` service: a record for every file listed, with no IO and no ports.

use crate::domain::workers::types::Fresh;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusEntry, FileNumber};
use ryo_values::domain::records::types::{FileOutcome, Record, Unreadable};
use ryo_values::domain::stored_facts::types::Digest;

/// A record for every file listed, in the order of the files: the one this run made, or, for a file no rule governs, its path and
/// the system's stamps alone, so that the next run knows it is as it was. A file whose bytes could not be read, one of `failed` in
/// the order of their paths, has none: the next run reads it again.
#[must_use]
pub(crate) fn every_file(files: &[CorpusEntry], fresh: Fresh, failed: &[FilePath]) -> Vec<Record> {
    let mut records = Vec::with_capacity(files.len());
    let mut made = fresh.places.into_iter().zip(fresh.records).peekable();
    for (place, file) in files.iter().enumerate() {
        if let Some((_, record)) = made.next_if(|(at, _)| at.0 == place) {
            records.push(record);
            continue;
        }
        if failed.binary_search(&file.path).is_err() {
            records.push(Record {
                path: file.path.clone(),
                size: file.size,
                modified: file.modified,
                digest: Digest([0; 16]),
                number: FileNumber(u32::try_from(place).unwrap_or(u32::MAX)),
                outcome: FileOutcome::Unread {
                    reason: Unreadable::NoLanguage,
                    ran: None,
                },
            });
        }
    }
    records
}
