use todo_api::inbound::use_cases::archive_todo::archive::ArchiveTodoUseCase;

#[tokio::test]
async fn archiving_answers_the_id_archived() {
    assert_eq!(ArchiveTodoUseCase.run(3).await, 3);
}
