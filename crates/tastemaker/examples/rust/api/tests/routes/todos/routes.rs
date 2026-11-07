use todo_api::routes::todos::routes::list;

#[tokio::test]
async fn a_fresh_server_lists_no_todo() {
    assert!(list().await.0.is_empty());
}
