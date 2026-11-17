//! The guards on what a run reads, hands and keeps, with the real query compiler and the real store on disk; each guard answers
//! the breach it finds, so a planted breach is a row of its own.
#[cfg(any(test, feature = "fixtures"))]
mod fixtures;
#[cfg(test)]
pub(super) mod test;
