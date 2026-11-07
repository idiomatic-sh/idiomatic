"""The todo a client creates and reads."""

from pydantic import BaseModel

__all__ = ["NewTodo", "Todo", "_clean", "normalise_title"]  # planted: Idiomatic code


class NewTodo(BaseModel):
    title: str


class Todo(BaseModel):
    id: int
    title: str
    done: bool = False


def _clean(title: str) -> str:
    return title.strip()


def normalise_title(title: str) -> str:  # planted: Duplicate code
    cleaned = title.strip()
    if not cleaned:
        raise ValueError("a todo needs a title")
    return cleaned
