//! Project batch cancellation and phase resource lifetime.
#[cfg(test)]
mod identity;
#[cfg(test)]
mod sharing;
#[cfg(test)]
pub(super) mod test;

#[cfg(any(test, feature = "fixtures"))]
mod fixtures;
