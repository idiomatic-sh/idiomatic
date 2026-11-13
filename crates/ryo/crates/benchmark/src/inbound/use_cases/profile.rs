//! Record the heap and CPU profiles of a linter bench's worker when its command line asks for `profile-ryo`.
mod constants;
pub mod interface;
mod service;
pub mod types;
pub mod use_case;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
#[cfg(test)]
mod use_case_test;
