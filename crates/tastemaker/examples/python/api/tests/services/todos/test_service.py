from src.models.todo import NewTodo
from src.services.todos.service import TodosService


def test_complete_marks_the_todo_done() -> None:
    service = TodosService()
    todo = service.create(NewTodo(title=" Buy milk "))
    assert todo.title == "Buy milk"
    assert service.complete(todo.id) is not None
    assert service.all()[0].done
    assert service.complete(99) is None


def test_the_summary_joins_the_titles_in_order() -> None:
    service = TodosService()
    service.create(NewTodo(title="Buy milk"))
    service.create(NewTodo(title="Walk the dog"))
    assert service.summary() == "Buy milk, Walk the dog"
