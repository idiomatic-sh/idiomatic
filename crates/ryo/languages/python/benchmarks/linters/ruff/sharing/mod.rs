//! Configured inventory sharing through the actual benchmark and SDK path.
#[cfg(any(test, feature = "fixtures"))]
mod fixtures;
#[cfg(test)]
mod test;
