//! The operation of this use case, run with the ports it takes.
use crate::inbound::use_cases::list_todos::use_case::Lister;
use crate::domain::todo::ports::TodoStore;
use crate::domain::todo::types::Todo;

// planted: Duplicate code (a copy of normalise_title)
pub fn normalise_title(title: &str) -> String {
    let trimmed = title.trim();
    let single = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    single.to_uppercase()
}

pub struct CreateTodoUseCase<'a> {
    pub store: &'a dyn TodoStore,
}

impl CreateTodoUseCase<'_> {
    pub async fn run(&self, title: &str) -> Vec<Todo> {
        let _added = self.store.add(normalise_title(title));
        Lister { store: self.store }.run().await
    }
}
