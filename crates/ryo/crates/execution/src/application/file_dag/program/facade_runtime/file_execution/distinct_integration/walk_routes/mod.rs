//! Actual shared-source consumers cross row, observation and selected-publication barriers.
#[cfg(test)]
mod continuation;
#[cfg(test)]
mod derived_barrier;
#[cfg(test)]
mod distinct_gate;
#[cfg(any(test, feature = "fixtures"))]
mod fixtures;
#[cfg(test)]
mod lifetime;
#[cfg(test)]
mod observation;
#[cfg(test)]
mod selection;
