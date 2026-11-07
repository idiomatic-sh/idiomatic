"""Every kind of binding."""
import os.path
from pkg import a as b
from m import *

__all__ = ["run", "Model"]
counter = 0


def run(first, /, second=1, *args, third, **kwargs):
    global counter
    counter += 1
    for index, value in enumerate(args):
        pass
    with open(first) as handle:
        pass
    try:
        pass
    except* ValueError as error:
        pass
    if (found := second):
        pass
    return [(last := item) for item in args]


def outer():
    state = 0

    def inner():
        nonlocal state
        state = 1


class Model[T, *Ts, **P]:
    async def save(self) -> None:
        pass


type Pair[K] = tuple[K, K]

match command:
    case Point(x=0, y=y) as point:
        pass
    case [first_item, *others]:
        pass
    case {"key": value, **extra}:
        pass
