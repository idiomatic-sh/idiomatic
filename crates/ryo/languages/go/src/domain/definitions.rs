//! The go domain's `definitions` concept: what an element declares, as the `defines` and `annotates` relations say it: its shared
//! kind, Go's word for it, its name and the scope that binds it.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
