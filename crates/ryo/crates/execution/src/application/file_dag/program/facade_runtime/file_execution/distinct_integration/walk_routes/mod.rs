//! Actual shared-source consumers cross row, observation and selected-publication barriers.
#[cfg(test)]
mod continuation;
#[cfg(any(test, feature = "fixtures"))]
mod fixtures;
#[cfg(test)]
mod observation;
#[cfg(test)]
mod selection;
