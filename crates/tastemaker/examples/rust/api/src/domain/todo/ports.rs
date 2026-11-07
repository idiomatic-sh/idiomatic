use crate::domain::todo::types::Todo;

/// Where todos are kept.
pub trait TodoStore: Send + Sync {
    fn all(&self) -> Vec<Todo>;
    fn add(&self, title: String) -> Todo;
    fn complete(&self, id: u64) -> Option<Todo>;
}
