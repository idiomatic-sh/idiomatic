"""What the tag adapter of `pending` promises beside the port's contract, proven
without running the job it wraps."""

from __future__ import annotations

from ci.changes.pending.adapters.tag.adapter import TagPending
from ci.changes.pending.interface_test import conforms
from ci.tag.use_case import Request


class Job:
    """The `tag` job's inbound call, recorded: it answers the products it would release."""

    def __init__(self, released: tuple[str, ...]) -> None:
        self.released = released
        self.asked: list[Request] = []

    def run(self, request: Request) -> tuple[str, ...]:
        self.asked.append(request)
        return self.released


def test_the_tag_adapter_implements_the_port() -> None:
    """It is a `pending` adapter."""
    conforms(TagPending)


def test_the_unreleased_products_are_the_ones_the_tag_job_would_tag_at_the_checkout_without_pushing() -> None:
    job = Job(("ryo", "tastemaker"))
    assert TagPending(job).unreleased() == frozenset({"ryo", "tastemaker"})  # type: ignore[arg-type]
    assert job.asked == [Request("HEAD", push=False)]
