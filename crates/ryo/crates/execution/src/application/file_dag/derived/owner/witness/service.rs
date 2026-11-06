//! Opaque observation cannot upgrade or detach a semantic payload allocation.
use std::{any::Any, sync::Weak};
#[derive(Clone)]
pub(in crate::application::file_dag) struct BackingWitness {
    backing: Weak<dyn Any>,
}
impl BackingWitness {
    pub(in crate::application::file_dag::derived::owner) fn new(backing: Weak<dyn Any>) -> Self {
        Self { backing }
    }
    pub(in crate::application::file_dag) fn is_alive(&self) -> bool {
        self.backing.strong_count() != 0
    }
    pub(in crate::application::file_dag) fn same_backing(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.backing, &other.backing)
    }
}
