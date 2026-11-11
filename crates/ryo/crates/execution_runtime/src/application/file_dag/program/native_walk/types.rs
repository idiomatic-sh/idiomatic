//! The types of the `native_walk` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Physical delivery must decline before opening any shared predicate work.
#[derive(Clone, Copy)]
pub(crate) enum WalkDelivery {
    Rows,
}

/// Entry-point calls a rule has yet to make.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Deliveries(pub(crate) usize);

/// What retirement decides for one batch.
pub(super) enum Fate {
    Keep,
    Release,
}

/// Batches a file walk still holds.
#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RetainedBatches(pub(crate) usize);
