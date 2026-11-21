//! The types of the `entries` part: the loop's three entries behind one name.

use crate::domain::code::types::CodeEntry;
use crate::domain::executing::bodies::service::asked_pattern;
use crate::domain::executing::service::run;
use crate::domain::executing::sources::service::table_needed;
use crate::domain::findings::types::InstructionCount;
use crate::domain::made_tables::types::PatternAsk;
use crate::domain::memos::types::MemoId;
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::frames::types::{LoopEntry, RunMode, StepFrame};
use crate::domain::steps::types::MadeTableId;

/// The loop: what a frame carries as its ports.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TheLoop;

impl LoopEntry for TheLoop {
    fn run(
        &self,
        frame: &mut StepFrame<'_, '_>,
        entry: CodeEntry,
        mode: RunMode,
    ) -> InstructionCount {
        run(frame, entry, mode)
    }

    fn pattern(&self, frame: &mut StepFrame<'_, '_>, asked: PatternAsk<'_>) -> MemoId {
        asked_pattern(frame, asked)
    }

    fn table_need(&self, frame: &mut StepFrame<'_, '_>, table: MadeTableId) -> NeedVerdict {
        table_needed(frame, table)
    }
}
