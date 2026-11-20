//! The `declarations` part of the model: the `global` and `nonlocal` names of each scope, where they route a binding, and the
//! deletions that leave a name's origin unproven.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
