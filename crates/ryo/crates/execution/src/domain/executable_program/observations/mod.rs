//! One cohesive public-contract proof concept.
#[cfg(any(test, feature = "fixtures"))]
pub(in crate::domain) mod fixtures;
#[cfg(test)]
pub(super) mod test;
