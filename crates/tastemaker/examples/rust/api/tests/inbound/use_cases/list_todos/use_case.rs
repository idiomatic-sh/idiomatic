use todo_api::domain::todo::ports::TodoStore;
use todo_api::inbound::use_cases::list_todos::use_case::Lister;
use todo_api::outbound::todos::adapter::MemoryTodoStore;

#[tokio::test]
async fn every_kept_todo_is_listed() {
    let store = MemoryTodoStore::default();
    let added = store.add("Buy milk".to_owned());
    assert_eq!(Lister { store: &store }.run().await, vec![added]);
}
