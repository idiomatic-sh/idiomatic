use todo_api::domain::todo::ports::TodoStore;
use todo_api::inbound::use_cases::complete_todo::use_case::CompleteTodoUseCase;
use todo_api::outbound::todos::adapter::MemoryTodoStore;

#[tokio::test]
async fn a_kept_todo_is_completed() {
    let store = MemoryTodoStore::default();
    let added = store.add("Buy milk".to_owned());
    let completed = CompleteTodoUseCase { store: &store }.run(added.id).await;
    assert!(completed.into_iter().any(|todo| todo.done && todo.id == added.id));
}

#[tokio::test]
async fn a_todo_that_is_not_kept_is_not_completed() {
    let store = MemoryTodoStore::default();
    let completed = CompleteTodoUseCase { store: &store }.run(42).await;
    assert!(completed.into_iter().next().is_none());
}
