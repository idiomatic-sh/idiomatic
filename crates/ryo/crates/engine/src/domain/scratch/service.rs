//! The `scratch` service: a worker's per-file state cleared between files.

use super::types::ScanScratch;
use ryo_language::domain::facts::service::clear_words;

/// Forgets what the last file left: its filled relations and columns, made tables, registers and words, keeping their room.
pub(crate) fn clear_file(scratch: &mut ScanScratch) {
    let state = &mut scratch.0;
    state.registers.clear();
    state.filled.clear();
    for columns in &mut state.columns {
        columns.0.clear();
    }
    for rows in &mut state.made {
        rows.clear();
    }
    clear_words(&mut state.words);
    state.present.0.clear();
}
