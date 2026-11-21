//! The cases of the `code` service, by part: every code's constructor read back by its reader, every per-row meaning read
//! back through both call words, the pool's records, and the records of the columns a graph fill's kernel is told.
mod anchoring;
mod calls;
mod controlling;
mod deciding;
mod filling;
#[cfg(test)]
mod fixtures;
mod graph_columns;
mod records;
mod walking;
