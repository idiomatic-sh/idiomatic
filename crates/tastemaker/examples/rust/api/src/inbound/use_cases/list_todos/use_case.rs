//! The operation of this use case, run with the ports it takes.
use crate::inbound::use_cases::create_todo::use_case::CreateTodoUseCase;
use crate::domain::todo::ports::TodoStore;
use crate::domain::todo::types::Todo;

// planted: Naming convention, and the import back to create_todo closes the cycle (planted: Dependency graph)
pub struct Lister<'a> {
    pub store: &'a dyn TodoStore,
}

impl<'a> Lister<'a> {
    pub async fn run(&self) -> Vec<Todo> {
        self.store.all()
    }

    pub fn creator(&self) -> CreateTodoUseCase<'a> {
        CreateTodoUseCase { store: self.store }
    }
}
