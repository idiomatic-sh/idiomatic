//! The `reporting` service: free functions named by verb that put the rows of the cross-file steps in order after the rows of the
//! files and count them, and report the files left unread, with no IO and no ports.

use ryo_language::domain::symbols::service::file_in;
use ryo_values::domain::row_texts::types::RowTotal;

use super::types::RowCounts;
use crate::domain::finish::types::FoundRow;

use crate::domain::keeping::service::rows_from;
use ryo_execution::inbound::use_cases::metadata::interface::Message;
use ryo_execution::inbound::use_cases::metadata::interface::Unknown;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::records::types::Unread;
use ryo_values::domain::records::types::Unreadable;
use ryo_values::domain::row_texts::types::FindingRows;
use ryo_values::domain::runs::types::ManifestRefused;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::ReadFailure;
use ryo_values::domain::stored_findings::types::FindingMade;
use ryo_values::domain::stored_findings::types::StoredFinding;
use ryo_values::domain::stores::types::Stored;

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

/// The findings of the rules that read across files, as the store kept them; none when a finding's rule is not one of `ids`.
#[must_use]
pub(crate) fn later_from(
    stored: &Stored,
    ids: &[RuleId],
    made_again: &[RuleId],
) -> Option<FindingRows> {
    let across: Vec<&StoredFinding> = stored
        .findings
        .iter()
        .filter(|finding| {
            finding.made == FindingMade::AcrossFiles
                && made_again.iter().all(|rule| *rule.0 != *finding.rule.0)
        })
        .collect();
    rows_from(&across, ids)
}

/// What the rules could not decide, as the store kept it, but in the files read again, whose own say now.
#[must_use]
pub(crate) fn unknowns_from(stored: &Stored, read_again: &[FilePath]) -> Vec<Unknown> {
    stored
        .unknowns
        .iter()
        .filter(|each| read_again.binary_search(&each.path).is_err())
        .map(|each| Unknown {
            rule: RuleId(each.rule.0.clone()),
            path: each.path.clone(),
            message: Message(each.message.0.clone()),
            cause: each.cause,
        })
        .collect()
}

/// The files read again, and with them the files the store kept that the run no longer lists: they are gone, so what the store kept
/// of them answers nothing about this run. Sorted, once each, as `unknowns_from` and `skipped_from` search them.
#[must_use]
pub(crate) fn with_gone(
    mut read_again: Vec<FilePath>,
    stored: &Stored,
    listed: &[&FilePath],
) -> Vec<FilePath> {
    let listed: std::collections::BTreeSet<&FilePath> = listed.iter().copied().collect();
    read_again.extend(
        stored
            .files
            .iter()
            .filter(|kept| !listed.contains(&kept.path))
            .map(|kept| kept.path.clone()),
    );
    read_again.sort();
    read_again.dedup();
    read_again
}

/// What the rules explicitly skipped, as the store kept it, but not for files read again.
#[must_use]
pub(crate) fn skipped_from(stored: &Stored, read_again: &[FilePath]) -> Vec<Skipped> {
    stored
        .skipped
        .iter()
        .filter(|each| read_again.binary_search(&each.path).is_err())
        .cloned()
        .collect()
}
