//! Run the candidate child through injected transport and SDK boundaries.
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod interface;
pub mod use_case;

#[cfg(test)]
mod use_case_test;
