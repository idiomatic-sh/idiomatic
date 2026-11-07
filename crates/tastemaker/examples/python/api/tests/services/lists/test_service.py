from src.models.todo import NewTodo, Todo
from src.services.lists.service import ListsService
from src.services.todos.service import TodosService


def test_titles_keep_the_order() -> None:
    todos = [Todo(id=1, title="a"), Todo(id=2, title="b")]
    assert ListsService().titles(todos) == ["a", "b"]


def test_the_open_titles_are_those_not_done() -> None:
    service = TodosService()
    done = service.create(NewTodo(title="a"))
    service.create(NewTodo(title="b"))
    service.complete(done.id)
    assert ListsService().open_titles(service) == ["b"]
