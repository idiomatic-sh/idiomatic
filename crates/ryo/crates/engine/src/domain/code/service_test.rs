//! The cases of the `code` service, by part: every code's constructor read back by its reader, every per-row meaning read
//! back through both call words, and the pool's records.
mod anchoring;
mod calls;
mod controlling;
mod deciding;
mod filling;
#[cfg(test)]
mod fixtures;
mod records;
mod walking;
