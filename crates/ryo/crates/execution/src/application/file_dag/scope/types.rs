//! The scope namespace alone issues production owner-creation authority.
/// A permit's private field prevents unrelated application code from creating a
/// RelationOwner directly. Only this cohesive outer scope constructor mints it.
pub(in crate::application::file_dag) struct ScopePermit {
    _private: (),
}
#[cfg(test)]
impl ScopePermit {
    pub(in crate::application::file_dag) fn fixture() -> Self {
        Self { _private: () }
    }
}

impl ScopePermit {
    pub(super) fn new() -> Self {
        Self { _private: () }
    }
}
