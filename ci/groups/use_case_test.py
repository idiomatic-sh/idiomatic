"""What the `groups` job's use case promises, over fakes of every port."""

from __future__ import annotations

import pytest

from ci.groups.manifest.interface import Manifest
from ci.groups.use_case import Refused, Request, UseCase


class FakeManifest(Manifest):
    """Each product's declared groups; a product missing from the table has no manifest."""

    def __init__(self, declared: dict[str, set[str]]) -> None:
        self.declared = declared

    def groups(self, product: str) -> frozenset[str] | None:
        return frozenset(self.declared[product]) if product in self.declared else None


# ryo and Tastemaker declare `memory`; asymptote does not.
DECLARED = {"ryo": {"lint", "check", "memory"}, "tastemaker": {"lint", "check", "memory"}, "asymptote": {"lint", "check"}}


def kept(product: str, *asked: str, declared: dict[str, set[str]] = DECLARED) -> tuple[str, ...]:
    return UseCase(FakeManifest(declared)).run(Request(product, asked))


def test_a_product_is_asked_only_the_groups_it_declares_in_the_order_asked() -> None:
    assert kept("asymptote", "memory", "check", "lint") == ("check", "lint")
    assert kept("ryo", "memory", "check", "lint") == ("memory", "check", "lint")


def test_a_group_no_product_declares_is_refused_not_dropped() -> None:
    with pytest.raises(Refused, match="^no product declares nightly$"):
        kept("ryo", "lint", "nightly")


def test_a_product_left_with_no_group_is_refused() -> None:
    with pytest.raises(Refused, match="^asymptote declares none of memory$"):
        kept("asymptote", "memory")


def test_a_product_without_its_manifest_is_refused() -> None:
    with pytest.raises(Refused, match="^tastemaker has no crates/tastemaker/asymptote.toml$"):
        kept("tastemaker", "lint", declared={"ryo": {"lint"}, "asymptote": {"lint"}})


def test_a_name_that_is_no_product_is_refused() -> None:
    with pytest.raises(Refused, match="^nope is not a product: ryo, tastemaker, asymptote$"):
        kept("nope", "lint")
