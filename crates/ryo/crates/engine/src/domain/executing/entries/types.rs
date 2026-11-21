//! The types of the `entries` part: the loop's three entries behind one name.

use crate::domain::code::types::CodeEntry;
use crate::domain::executing::kept::service::asked_pattern;
use crate::domain::executing::measuring::types::ByPoint;
use crate::domain::executing::service::{run, run_counted};
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

/// The loop of a measuring worker: the same three entries, each word counted at its plan point.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TheMeasuringLoop;

impl LoopEntry for TheMeasuringLoop {
    fn run(
        &self,
        frame: &mut StepFrame<'_, '_>,
        entry: CodeEntry,
        mode: RunMode,
    ) -> InstructionCount {
        run_counted::<ByPoint>(frame, entry, mode)
    }

    fn pattern(&self, frame: &mut StepFrame<'_, '_>, asked: PatternAsk<'_>) -> MemoId {
        asked_pattern(frame, asked)
    }

    fn table_need(&self, frame: &mut StepFrame<'_, '_>, table: MadeTableId) -> NeedVerdict {
        table_needed(frame, table)
    }
}
