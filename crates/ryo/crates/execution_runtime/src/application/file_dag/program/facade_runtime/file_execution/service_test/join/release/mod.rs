//! Cancellation and primary-error release for the physical kernel strategies.
#[cfg(any(test, feature = "fixtures"))]
mod fixtures;
#[cfg(test)]
pub(super) mod test;
