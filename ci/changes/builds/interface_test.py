"""The contract every `builds` adapter meets: each adapter's test calls `conforms`."""

from __future__ import annotations

import inspect

from ci.changes.builds.interface import Builds


def conforms(adapter: type) -> None:
    """`adapter` implements the port: it subclasses it and defines each of the port's
    methods with the port's own signature."""
    assert Builds in adapter.__mro__, f"{adapter.__name__} does not implement Builds"
    for name, method in inspect.getmembers(Builds, inspect.isfunction):
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
