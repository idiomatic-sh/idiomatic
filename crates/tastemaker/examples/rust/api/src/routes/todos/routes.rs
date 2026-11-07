use axum::Json;

use crate::domain::todo::types::Todo;
use crate::inbound::use_cases::list_todos::use_case::Lister;
use crate::outbound::todos::adapter::MemoryTodoStore;

pub async fn list() -> Json<Vec<Todo>> {
    let store = MemoryTodoStore::default();
    Json(Lister { store: &store }.run().await)
}

// planted: Framework conventions (a handler that is not async)
pub fn count() -> Json<usize> {
    Json(0)
}
