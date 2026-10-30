from typing import Optional


def greet(name: str, times: int = 1, *rest: "Extra", **opts: Optional[int]) -> list[str]:
    result: list[str] = []
    count: int
    return result


class Config:
    debug: bool = False
    path: "pathlib.Path"
