//! The `handing` service: free functions named by verb over what a file handed the steps that read across files and the rows the
//! store keeps of it, with no IO and no ports.

use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::records::types::{Handed, HandedNotes};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{
    BucketsWanted, Stored, StoredCovered, StoredFileNumber, StoredHanded, StoredNotes, StoredText,
    StoredWords,
};

/// The rows of `rows` of the stored file `file`: the tables keep each file's rows together, in the order of the files.
pub(crate) fn rows_of_file<T>(
    rows: &[T],
    file: StoredFileNumber,
    of: impl Fn(&T) -> StoredFileNumber,
) -> &[T] {
    let start = rows.partition_point(|each| of(each) < file);
    let end = rows.partition_point(|each| of(each) <= file);
    rows.get(start..end).unwrap_or_default()
}

/// What the file at `path` handed the steps that read across files when the store was kept but its words, which the store keeps
/// where they are; nothing when the store has no row of it.
#[must_use]
pub(crate) fn handed_in(stored: &Stored, handed: &StoredHanded, path: &FilePath) -> Handed {
    let Ok(place) = stored.files.binary_search_by(|file| file.path.cmp(path)) else {
        return Handed::default();
    };
    let file = StoredFileNumber(u32::try_from(place).unwrap_or(u32::MAX));
    let covered = rows_of_file(&handed.covered, file, |each| each.file).first();
    Handed {
        buckets: Vec::new(),
        text: rows_of_file(&handed.texts, file, |each| each.file)
            .first()
            .map(|each| each.text.clone())
            .unwrap_or_default(),
        notes: rows_of_file(&handed.notes, file, |each| each.file)
            .first()
            .map_or(HandedNotes::Nothing, |each| {
                HandedNotes::Noted(each.notes.clone())
            }),
        governing: covered
            .map(|each| each.governing.clone())
            .unwrap_or_default(),
        partials: covered
            .map(|each| each.partials.clone())
            .unwrap_or_default(),
    }
}

/// Puts what the file `file` handed the steps that read across files in the store's tables, a row only where there is something.
pub(crate) fn hand_into(
    stored: &mut StoredHanded,
    file: StoredFileNumber,
    handed: Handed,
    rebuilt: &BucketsWanted,
) {
    let wanted = |bucket: Bucket| match rebuilt {
        BucketsWanted::All => true,
        BucketsWanted::These(buckets) => buckets.binary_search(&bucket).is_ok(),
    };
    stored.frontier.extend(
        handed
            .buckets
            .into_iter()
            .filter(|each| wanted(each.bucket))
            .map(|each| StoredWords {
                file,
                bucket: each.bucket,
                words: each.words,
            }),
    );
    if !handed.text.0.is_empty() {
        stored.texts.push(StoredText {
            file,
            text: handed.text,
        });
    }
    if let HandedNotes::Noted(notes) = handed.notes {
        stored.notes.push(StoredNotes { file, notes });
    }
    if !handed.governing.0.is_empty() {
        stored.covered.push(StoredCovered {
            file,
            governing: handed.governing,
            partials: handed.partials,
        });
    }
}
