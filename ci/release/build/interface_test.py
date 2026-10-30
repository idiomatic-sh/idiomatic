"""The contract every `build` adapter meets: each adapter's test calls `conforms`."""

from __future__ import annotations

import inspect

from ci.release.build.interface import Build


def conforms(adapter: type) -> None:
    """`adapter` implements the port: it subclasses it and defines each of the port's
    methods with the port's own signature."""
    assert Build in adapter.__mro__, f"{adapter.__name__} does not implement Build"
    for name, method in inspect.getmembers(Build, inspect.isfunction):
        if not name.startswith("_"):
            assert inspect.signature(getattr(adapter, name)) == inspect.signature(method), f"{adapter.__name__}.{name}"


def test_a_class_that_is_not_the_ports_does_not_conform() -> None:
    """The contract refuses a class that does not implement the port."""

    class Elsewhere:
        pass

    try:
        conforms(Elsewhere)
    except AssertionError:
        return
    raise AssertionError("conforms accepted a class that does not implement the port")
