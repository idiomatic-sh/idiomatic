//! The `model` service: free functions named by verb over the file model, with no IO and no ports.

use super::types::ReadIndexes;
use crate::domain::bindings::service::clear_bindings;

/// Empties every index of `model`, so that the next file's readers work them out again; the memory each holds is kept.
pub(crate) fn clear(model: &mut ReadIndexes) {
    clear_bindings(model.bindings.get_mut());
}
