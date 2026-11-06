//! The typescript domain's `fields` concept: the role of every element under its parent, in the words of the TypeScript-ESTree
//! properties.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;

#[cfg(test)]
mod fixtures;

#[cfg(test)]
mod positions_test;

#[cfg(test)]
mod jsx_test;

#[cfg(test)]
mod modules_test;

#[cfg(test)]
mod control_test;

#[cfg(test)]
mod types_test;

#[cfg(test)]
mod classes_test;
