//! The `reporting` service: free functions named by verb that put the rows of the cross-file steps in order after the rows of the
//! files and count them, and report the files left unread, with no IO and no ports.

use ryo_language::domain::symbols::service::file_in;
use ryo_values::domain::row_texts::types::RowTotal;

use super::types::RowCounts;
use crate::domain::finish::types::FoundRow;

use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::records::types::Unread;
use ryo_values::domain::records::types::Unreadable;
use ryo_values::domain::runs::types::ManifestRefused;
use ryo_values::domain::stored_facts::types::ReadFailure;

/// Puts the findings in the order of their places: by path, then line, then character.
pub(crate) fn order_found(found: &mut [FoundRow], paths: &[FilePath]) {
    found.sort_by(|left, right| {
        let path = |each: &FoundRow| file_in(paths, each.file).map(|path| &*path.0);
        path(left)
            .cmp(&path(right))
            .then(left.place.line.cmp(&right.place.line))
            .then(left.place.column.cmp(&right.place.column))
    });
}

/// How many more rows may be written once `shown` of the `limit` were.
#[must_use]
pub(crate) fn room_left(limit: RowTotal, shown: RowTotal) -> RowTotal {
    RowTotal(limit.0.saturating_sub(shown.0))
}

/// What the run counts once the `later` rows of the cross-file steps are added to what the files counted: every row is counted, and
/// those up to `room` are written.
#[must_use]
pub(crate) fn totals(files: RowCounts, later: RowTotal, room: RowTotal) -> RowCounts {
    RowCounts {
        total: RowTotal(files.total.0 + later.0),
        shown: RowTotal(files.shown.0 + later.0.min(room.0)),
    }
}

/// A manifest the tools refused is a finding of the run, as a file nothing could read is: it is reported, never a stop. Puts the
/// refused manifests among the files left unread, in the order of the paths.
pub(crate) fn report_refused(unread: &mut Vec<Unread>, refused: &[ManifestRefused]) {
    unread.extend(refused.iter().map(|each| Unread {
        path: each.path.clone(),
        reason: Unreadable::Failed(ReadFailure(each.reason.0.clone())),
    }));
    unread.sort_by(|left, right| left.path.cmp(&right.path));
}

/// Collect retry paths before adding refused manifests to the final unread report.
pub(crate) fn read_outcomes(
    unread: &mut Vec<Unread>,
    unrecorded: Vec<FilePath>,
    refused: &[ManifestRefused],
) -> Vec<FilePath> {
    let mut failed: Vec<FilePath> = unread
        .iter()
        .filter(|each| matches!(each.reason, Unreadable::Failed(_)))
        .map(|each| each.path.clone())
        .collect();
    failed.extend(unrecorded);
    failed.sort_unstable();
    failed.dedup();
    report_refused(unread, refused);
    failed
}

/// Preserve one ordered occurrence of each completed coverage record.
pub(crate) fn sort_unique<T>(records: &mut Vec<T>)
where
    T: Ord,
{
    records.sort();
    records.dedup();
}
