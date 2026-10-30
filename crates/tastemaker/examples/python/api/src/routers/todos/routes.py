"""The routes of the todos resource."""

from fastapi import APIRouter, HTTPException

from src.models.todo import NewTodo, Todo
from src.services.todos.service import TodosService

__all__ = ["complete_todo", "count_todos", "create_todo", "list_todos", "router"]

router = APIRouter(prefix="/todos", tags=["todos"])
_service = TodosService()


@router.post("")
async def create_todo(new: NewTodo) -> Todo:
    return _service.create(new)


@router.get("")
async def list_todos() -> list[Todo]:
    return _service.all()


@router.post("/{todo_id}/complete")
async def complete_todo(todo_id: int) -> Todo:
    todo = _service.complete(todo_id)
    if todo is None:
        raise HTTPException(status_code=404, detail="no such todo")
    return todo


@router.get("/count")
def count_todos() -> int:  # planted: Framework conventions
    return len(_service.all())
