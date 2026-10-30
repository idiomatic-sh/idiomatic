# planted: Required tests
"""The user a todo belongs to: it has no test file under tests/models/."""

from pydantic import BaseModel

__all__ = ["User"]


class User(BaseModel):
    id: int
    name: str
