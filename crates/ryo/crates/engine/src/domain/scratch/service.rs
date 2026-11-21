//! The `scratch` service: a worker's per-file state reset between files, without an allocation: the fill side's store is
//! stamped with the new file, and every arena of the loop is emptied, keeping its room.

use super::types::{BodyAnswer, LoopWork, ScanScratch, SeamRows};
use crate::domain::made_tables::sources::service::next_file as next_store;
use crate::domain::memos::service::clear_memos;
use ryo_language::domain::facts::service::clear_words;

/// Starts a file: nothing the last file left is read again.
pub(crate) fn next_file(scratch: &mut ScanScratch) {
    let engine = &mut scratch.0;
    next_store(&mut engine.store);
    clear_words(&mut engine.words);
    let state = &mut engine.state;
    clear_memos(&mut state.memos);
    state.site_answers.clear();
    state.frames.clear();
    state.yielded.clear();
    state.staged.clear();
    state.key.clear();
    state.needs.clear();
    state.rows = SeamRows::default();
    state.answer = BodyAnswer::default();
    state.fault = None;
    state.work = LoopWork::default();
    state.budget.reaching.clear();
    state.budget.pending.clear();
    state.budget.asks.clear();
    state.budget.asked.clear();
    state.budget.sums.clear();
    state.filling = Default::default();
    state.budget.sum_units.clear();
}
