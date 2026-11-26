"""Strings in an annotation that are values (the metadata of `Annotated`, the arguments of a call, `Literal`), and those that are types."""
import typing
import typing as t
from typing import Annotated, Literal, Optional

from fastapi import FastAPI
from pydantic import BaseModel, Field, WithJsonSchema

app = FastAPI()


class Item(BaseModel):
    name: str
    sub_items: list["Item"] = []
    state: Literal["pending", "success", "error"] = "pending"
    kind: typing.Literal["a"] = "a"
    other: t.Literal["b"] = "b"
    description: Annotated[
        Optional[str], WithJsonSchema({"type": ["string", "null"]})
    ] = None
    limit: Annotated["Item", Field(description="a value")] = None


@app.get("/no-content")
def return_no_content() -> "None":
    return
