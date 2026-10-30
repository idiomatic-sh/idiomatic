use std::sync::Mutex;

use crate::domain::todo::ports::TodoStore;
// planted: Idiomatic code (a wildcard import)
use crate::domain::todo::types::*;

/// Keeps the todos in memory.
#[derive(Default)]
pub struct MemoryTodoStore {
    todos: Mutex<Vec<Todo>>,
}

impl TodoStore for MemoryTodoStore {
    fn all(&self) -> Vec<Todo> {
        self.todos.lock().map(|todos| todos.clone()).unwrap_or_default()
    }

    fn add(&self, title: String) -> Todo {
        let mut todos = self.todos.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let todo = Todo { id: todo_shared::next_id(todos.len() as u64), title, done: false };
        todos.push(todo.clone());
        todo
    }

    fn complete(&self, id: u64) -> Option<Todo> {
        let mut todos = self.todos.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let found = todos.iter_mut().find(|todo| todo.id == id)?;
        found.done = true;
        Some(found.clone())
    }
}
