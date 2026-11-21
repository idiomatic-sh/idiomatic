//! The types of the `measuring` part: the counter the one loop is generic over.

use crate::domain::code::types::CodeEntry;
use crate::domain::executing::measuring::service::word_run;
use crate::domain::programs::frames::types::StepFrame;

/// What the loop tells of each word it runs. The plain loop tells nothing and holds no count of words by plan point;
/// the measuring loop counts each word at its plan point. One loop, two instances.
pub(crate) trait WordCounter {
    /// The word at `at` is about to run.
    fn word(frame: &mut StepFrame<'_, '_>, at: CodeEntry);
}

/// The counter of a worker that does not measure.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Uncounted;

impl WordCounter for Uncounted {
    #[inline]
    fn word(_frame: &mut StepFrame<'_, '_>, _at: CodeEntry) {}
}

/// The counter of a measuring worker.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ByPoint;

impl WordCounter for ByPoint {
    #[inline]
    fn word(frame: &mut StepFrame<'_, '_>, at: CodeEntry) {
        word_run(frame, at);
    }
}
