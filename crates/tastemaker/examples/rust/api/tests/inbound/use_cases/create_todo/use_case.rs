use todo_api::inbound::use_cases::create_todo::use_case::CreateTodoUseCase;
use todo_api::outbound::todos::adapter::MemoryTodoStore;

#[tokio::test]
async fn a_created_todo_is_listed_with_its_title_normalised() {
    let store = MemoryTodoStore::default();
    let listed = CreateTodoUseCase { store: &store }.run("  buy   milk ").await;
    let titles: Vec<&str> = listed.iter().map(|todo| todo.title.as_str()).collect();
    assert_eq!(titles, ["BUY MILK"]);
}
