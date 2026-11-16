//! The one CPU limit of every shape, and how many lints decide a size's time.
use ryo_benchmark_contracts::domain::measurements::types::DoublingRatio;

/// The most a shape's CPU time may grow when its program doubles, over the floored half-size time.
pub(super) const DOUBLING_LIMIT: DoublingRatio = DoublingRatio(2.2);
/// How many lints of each size a shape takes; the fastest is the size's time, as a busy machine slows a run and never
/// speeds it.
pub(super) const LINTS: usize = 5;
/// A thousand: the memory bound's per-thousand step, and the bytes of a kilobyte.
pub(super) const THOUSAND: u64 = 1_000;
/// The prefix of each program's scene directory.
pub(super) const SCENE: &str = "scene-";
/// The refusal of a plan with no shape: a run that grew nothing would hold every bound.
pub(super) const NO_SHAPE: &str = "the growth plan holds no shape";
