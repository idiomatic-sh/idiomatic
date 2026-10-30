use todo_api::domain::todo::ports::TodoStore;
use todo_api::outbound::todos::adapter::MemoryTodoStore;

#[test]
fn each_todo_gets_its_own_id_and_completes() {
    let store = MemoryTodoStore::default();
    let first = store.add("Buy milk".to_owned());
    let second = store.add("Call ann".to_owned());
    assert_ne!(first.id, second.id);
    assert_eq!(store.complete(second.id).map(|todo| todo.done), Some(true));
    assert_eq!(store.complete(99), None);
}
