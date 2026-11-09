//! Compare two contestants rule by rule over one corpus, judged by the differences accepted for it.
mod constants;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod interface;
mod service;
pub mod types;
pub mod use_case;

#[cfg(test)]
mod use_case_test;
