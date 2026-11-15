//! How long a node's rows live in a file: until its last consumer, node reader or engine use, and never past the file.
#[cfg(test)]
mod fixtures;
#[cfg(test)]
pub(super) mod test;
