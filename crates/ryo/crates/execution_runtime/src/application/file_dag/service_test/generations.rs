//! Two file generation owners cannot reuse each other's results: each file's generation consults its own provider and keeps its output.
#[cfg(test)]
mod fixtures;
#[cfg(test)]
pub(super) mod test;
