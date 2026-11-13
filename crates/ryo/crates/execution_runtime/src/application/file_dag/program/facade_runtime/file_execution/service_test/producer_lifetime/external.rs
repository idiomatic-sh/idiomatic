//! Deduplicated external reader obligations and real required resources.
#[cfg(test)]
pub(super) mod test;

#[cfg(any(test, feature = "fixtures"))]
mod fixtures;
