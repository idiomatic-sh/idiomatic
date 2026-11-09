//! The current interpreter's own use-case tests: what it counts and holds while it runs, read through the runtime's fixture
//! accessors. No test outside this directory names it, and the rewrite deletes it whole.
#[cfg(test)]
mod axis_graph;
#[cfg(test)]
mod conformance_scale;
#[cfg(test)]
mod owner_lifetime;
