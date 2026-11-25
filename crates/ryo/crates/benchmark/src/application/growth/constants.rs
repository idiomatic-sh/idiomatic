//! The one CPU limit of every shape, and how many lints decide a size's time.
use benchmark_contracts::domain::measurements::types::DoublingRatio;

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
/// Between two failures of one growth run.
pub(super) const FAILURE_SEPARATOR: &str = "\n";
/// Where a template text writes its repetition's index.
pub(super) const INDEX: &str = "{i}";
/// Between the parts of a shape's name: rule, frame, operation and whole size.
pub(super) const SHAPE_SEPARATOR: &str = "/";
/// The refusal of an operation naming a frame its table does not define.
pub(super) const UNKNOWN_FRAME: &str = "a frame the growth table does not define";
