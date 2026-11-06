//! producer lifetime proof roles.
#[cfg(test)]
pub(super) mod test;

#[cfg(test)]
mod external;
#[cfg(any(test, feature = "fixtures"))]
mod fixtures;

#[cfg(test)]
mod project;
