//! Opaque observation cannot upgrade or detach a semantic payload allocation.
use std::{any::Any, sync::Weak};
/// Whether a backing is still held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackingLiveness {
    Alive,
    Released,
}
/// Whether two witnesses observe one backing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackingIdentity {
    Same,
    Different,
}
/// An observation of a view's backing that grants no access to it.
#[derive(Clone)]
pub struct BackingWitness {
    backing: Weak<dyn Any>,
}
impl BackingWitness {
    pub(crate) fn new(backing: Weak<dyn Any>) -> Self {
        Self { backing }
    }
    /// Whether the backing is still held.
    pub fn liveness(&self) -> BackingLiveness {
        if self.backing.strong_count() != 0 {
            BackingLiveness::Alive
        } else {
            BackingLiveness::Released
        }
    }
    /// Whether `other` observes the same backing.
    pub fn compare(&self, other: &Self) -> BackingIdentity {
        if Weak::ptr_eq(&self.backing, &other.backing) {
            BackingIdentity::Same
        } else {
            BackingIdentity::Different
        }
    }
}
