use todo_api::domain::todo::types::Todo;

#[test]
fn a_todo_compares_by_its_fields() {
    let todo = Todo { id: 1, title: "Buy milk".to_owned(), done: false };
    assert_eq!(todo.clone(), todo);
    assert_ne!(Todo { done: true, ..todo.clone() }, todo);
}
