use todo_api::domain::todo::ports::TodoStore;
use todo_api::outbound::todos::adapter::MemoryTodoStore;

#[test]
fn a_store_gives_back_what_it_was_given() {
    let store = MemoryTodoStore::default();
    let port: &dyn TodoStore = &store;
    let added = port.add("Buy milk".to_owned());
    assert_eq!(port.all(), vec![added]);
}
