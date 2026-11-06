//! One cohesive public-contract proof concept.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
#[cfg(test)]
pub(super) mod test;
