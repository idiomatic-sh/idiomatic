//! Rule outputs outlive workspace drops, and evaluation errors leave no mutable state behind.
#[cfg(test)]
mod fixtures;
#[cfg(test)]
pub(super) mod test;
