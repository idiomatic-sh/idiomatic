//! The `model` service: free functions named by verb over the file model, with no IO and no ports.

use super::types::ReadIndexes;
use ryo::languages;

/// Empties every index for the next file, keeping its capacity: no index of the last file is read for this one.
pub(crate) fn clear(indexes: &mut ReadIndexes) {
    languages::clear_keys(indexes.shapes.get_mut());
}
