//! Graph demand of one invocation, answered only through the cache of the file's owner.
pub(in crate::application::file_dag) mod service;
pub(crate) mod types;

#[cfg(test)]
mod test;
