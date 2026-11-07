import pytest

from src.models.todo import NewTodo, Todo, normalise_title


def test_a_new_todo_is_not_done() -> None:
    todo = Todo(id=1, title=NewTodo(title="Buy milk").title)
    assert not todo.done


def test_a_title_is_trimmed_and_an_empty_one_is_refused() -> None:
    assert normalise_title("  Buy milk ") == "Buy milk"
    with pytest.raises(ValueError):
        normalise_title("   ")
