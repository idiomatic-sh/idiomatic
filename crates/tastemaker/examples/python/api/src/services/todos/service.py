"""The todos service: the operations on todos, kept in memory."""

from src.models.todo import NewTodo, Todo
from src.services.lists.service import ListsService  # planted: Dependency graph

__all__ = ["TodosService", "normalise_title"]


class TodosService:
    def __init__(self) -> None:
        self._todos: list[Todo] = []

    def create(self, new: NewTodo) -> Todo:
        todo = Todo(id=len(self._todos) + 1, title=normalise_title(new.title))
        self._todos.append(todo)
        return todo

    def all(self) -> list[Todo]:
        return list(self._todos)

    def complete(self, todo_id: int) -> Todo | None:
        for todo in self._todos:
            if todo.id == todo_id:
                todo.done = True
                return todo
        return None

    def summary(self) -> str:
        return ", ".join(ListsService().titles(self._todos))


def normalise_title(title: str) -> str:  # planted: Duplicate code
    cleaned = title.strip()
    if not cleaned:
        raise ValueError("a todo needs a title")
    return cleaned
