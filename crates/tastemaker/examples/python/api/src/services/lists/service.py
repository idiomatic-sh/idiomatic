"""The lists service: the views of the todos a list shows."""

from typing import TYPE_CHECKING

from src.models.todo import Todo

if TYPE_CHECKING:
    from src.services.todos.service import TodosService  # planted: Dependency graph

__all__ = ["ListsService"]


class ListsService:
    def open_in(self, todos: list[Todo]) -> list[Todo]:
        return [todo for todo in todos if not todo.done]

    def titles(self, todos: list[Todo]) -> list[str]:
        return [todo.title for todo in todos]

    def open_titles(self, service: "TodosService") -> list[str]:
        return self.titles(self.open_in(service.all()))
