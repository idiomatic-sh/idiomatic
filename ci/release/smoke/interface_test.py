"""The contract every `smoke` adapter meets: each adapter's test calls `conforms`."""

from __future__ import annotations

import inspect

import pytest

from ci.release.smoke.interface import Smoke, version_matches
from ci.release.use_case import Product


@pytest.mark.parametrize(("printed", "matches"), [
    ("tm 0.1.0 (python 0.1.0)\n", True),
    ("tm 0.1.0 ()\n", True),
    ("tm 0.1.1 (python 0.1.0)\n", False),
    ("tm 0.1.0 (python 0.1.0\n", False),
    ("tm 0.1.0\n", False),
])
def test_tastemaker_reports_its_version_with_its_provider_catalog(printed, matches) -> None:
    product = Product("tastemaker", "idiomatic-sh-tastemaker", "0.1.0", "MIT", "", False, "tm", ())
    assert version_matches(product, printed) is matches


def conforms(adapter: type) -> None:
    """`adapter` implements the port: it subclasses it and defines each of the port's
    methods with the port's own signature."""
    assert Smoke in adapter.__mro__, f"{adapter.__name__} does not implement Smoke"
    for name, method in inspect.getmembers(Smoke, inspect.isfunction):
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
