//! The rows the engine holds for a run after the files, and what a worker keeps from one file to the next, read through
//! `plan` and `run`.
#[cfg(test)]
mod dormant;
#[cfg(test)]
mod engine_uses;
#[cfg(test)]
mod retained;
#[cfg(test)]
pub(super) mod test;
