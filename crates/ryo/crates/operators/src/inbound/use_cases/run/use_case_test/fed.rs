//! The rows a node was fed, through `plan` and `run` only, the node counters that sum them over files, the run budget that
//! holds what it wrote to them, and the growth harness that holds every operator's rows and work to its declaration.
#[cfg(test)]
mod budget;
#[cfg(test)]
mod counters;
#[cfg(test)]
mod growth;
#[cfg(test)]
pub(super) mod test;
