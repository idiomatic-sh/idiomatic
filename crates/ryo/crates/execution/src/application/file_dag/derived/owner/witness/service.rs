//! Opaque observation cannot upgrade or detach a semantic payload allocation.
use std::{any::Any, sync::Weak};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BackingLiveness {
    Alive,
    Released,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BackingIdentity {
    Same,
    Different,
}
#[derive(Clone)]
pub(crate) struct BackingWitness {
    backing: Weak<dyn Any>,
}
impl BackingWitness {
    pub(crate) fn new(backing: Weak<dyn Any>) -> Self {
        Self { backing }
    }
    pub(crate) fn liveness(&self) -> BackingLiveness {
        if self.backing.strong_count() != 0 {
            BackingLiveness::Alive
        } else {
            BackingLiveness::Released
        }
    }
    pub(crate) fn compare(&self, other: &Self) -> BackingIdentity {
        if Weak::ptr_eq(&self.backing, &other.backing) {
            BackingIdentity::Same
        } else {
            BackingIdentity::Different
        }
    }
}
