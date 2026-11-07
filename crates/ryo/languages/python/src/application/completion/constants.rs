//! The text the completion step fails with.

/// Why the completion step stops when the engine did not prepare the declarations model it is promised.
pub(super) const MODEL_PROMISED: &str =
    "Python completion requires the engine-prepared Declarations model";
