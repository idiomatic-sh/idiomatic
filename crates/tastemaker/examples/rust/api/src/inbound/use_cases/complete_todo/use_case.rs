//! The operation of this use case, run with the ports it takes.
use axum::http::StatusCode;
use crate::domain::todo::ports::TodoStore;
use crate::domain::todo::types::Todo;

// planted: Import direction (the use case imports the framework)
pub struct CompleteTodoUseCase<'a> {
    pub store: &'a dyn TodoStore,
}

impl CompleteTodoUseCase<'_> {
    pub async fn run(&self, id: u64) -> Result<Todo, StatusCode> {
        self.store.complete(id).ok_or(StatusCode::NOT_FOUND)
    }
}
