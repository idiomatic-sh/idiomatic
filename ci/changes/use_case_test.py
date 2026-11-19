"""What the `changes` job's use case promises, over fakes of every port."""

from __future__ import annotations

import pytest

from ci.changes.builds.interface import Builds
from ci.changes.diff.interface import Diff
from ci.changes.pending.interface import Pending
from ci.changes.use_case import CHECK_ROWS, Refused, Request, UseCase


class FakeDiff(Diff):
    """The paths the pull request changes."""

    def __init__(self, paths: tuple[str, ...]) -> None:
        self.changed = paths

    def paths(self) -> tuple[str, ...]:
        return self.changed


class FakeBuilds(Builds):
    """Tastemaker builds against ryo's tree, as its patch table says; no other product builds against another's."""

    def built_on(self, product: str) -> frozenset[str]:
        return frozenset({"ryo"}) if product == "tastemaker" else frozenset()


class FakePending(Pending):
    """The products whose version has no tag yet, and how often it was asked (it asks GitHub)."""

    def __init__(self, unreleased: frozenset[str]) -> None:
        self.answer = unreleased
        self.asked = 0

    def unreleased(self) -> frozenset[str]:
        self.asked += 1
        return self.answer


def job(*paths: str, unreleased: frozenset[str] = frozenset()) -> UseCase:
    return UseCase(FakeDiff(paths), FakeBuilds(), FakePending(unreleased))


def pull_request(*paths: str, unreleased: frozenset[str] = frozenset()):
    return job(*paths, unreleased=unreleased).run(Request("pull_request", ""))


def rows(plan) -> list[tuple[str, str, str]]:
    return [(row["product"], row["check"]["label"], row["check"]["runner"]) for row in plan.rows]


def every_row(product: str) -> list[tuple[str, str, str]]:
    return [(product, row["label"], row["runner"]) for row in CHECK_ROWS]


def test_a_change_to_the_root_s_documents_checks_no_product() -> None:
    plan = pull_request("README.md", "LICENSE", "_typos.toml")
    assert plan.products == ()
    assert plan.rows == ()
    assert plan.skips == ("check-linux", "check", "python-release", "bench")


def test_a_change_inside_one_product_checks_it_alone_with_every_release_row() -> None:
    plan = pull_request("crates/ryo/cli/src/main.rs", "crates/ryo/README.md")
    assert plan.products == ("ryo",)
    assert rows(plan) == every_row("ryo")
    assert plan.skips == ()


def test_the_jobs_of_one_product_skip_when_another_is_checked() -> None:
    plan = pull_request("crates/tastemaker/cli/src/main.rs")
    assert plan.products == ("tastemaker",)
    assert rows(plan) == every_row("tastemaker")
    assert plan.skips == ("python-release", "bench")


@pytest.mark.parametrize(
    "path",
    [".github/workflows/check.yml", "ci/changes/use_case.py", "tools/install.py", "rust-toolchain.toml", ".cargo/config.toml", "tastemaker/rules.toml"],
)
def test_a_shared_path_checks_every_product(path: str) -> None:
    plan = pull_request(path)
    assert plan.products == ("ryo", "tastemaker", "asymptote")
    assert rows(plan) == every_row("ryo") + every_row("tastemaker") + every_row("asymptote")
    assert plan.skips == ()


@pytest.mark.parametrize(
    ("path", "refusal"),
    [
        ("docs/guide.md", "docs/guide.md is neither shared nor the root's alone: name it in SHARED or ROOT_ONLY of ci/changes/use_case.py"),
        ("crates/nope/src/lib.rs", "crates/nope/src/lib.rs is in no product's directory: ryo, tastemaker, asymptote"),
        ("crates/ryo", "crates/ryo is in no product's directory: ryo, tastemaker, asymptote"),
        ("README.md/x", "README.md/x is neither shared nor the root's alone: name it in SHARED or ROOT_ONLY of ci/changes/use_case.py"),
    ],
)
def test_a_path_no_list_names_is_refused_not_guessed(path: str, refusal: str) -> None:
    with pytest.raises(Refused) as refused:
        pull_request("crates/ryo/cli/src/main.rs", path)
    assert str(refused.value) == refusal


def test_a_push_checks_every_product_and_a_hand_run_the_one_it_names() -> None:
    pushed = job().run(Request("push", ""))
    assert pushed.products == ("ryo", "tastemaker", "asymptote") and pushed.skips == ()
    handed = job().run(Request("workflow_dispatch", "asymptote"))
    assert handed.products == ("asymptote",)
    assert rows(handed) == every_row("asymptote")
    assert handed.skips == ("python-release", "bench")
    with pytest.raises(Refused) as refused:
        job().run(Request("workflow_dispatch", ""))
    assert str(refused.value) == "'' is not a product: ryo, tastemaker, asymptote"
    with pytest.raises(Refused) as refused:
        job().run(Request("schedule", ""))
    assert str(refused.value) == "schedule starts no check"


def test_a_product_built_on_an_unreleased_product_waits_for_its_resolution_row_alone() -> None:
    """ryo-gnv5: Tastemaker needs the ryo version this tree bumps, which only the release this check gates publishes."""
    plan = pull_request("crates/tastemaker/cli/src/main.rs", unreleased=frozenset({"ryo"}))
    assert rows(plan) == [each for each in every_row("tastemaker") if each[1] != "release-resolution"]
    assert plan.skips == ("python-release", "bench")
    # ryo's own resolution row stays: it builds on no other product's tree.
    shared = job(".github/workflows/check.yml", unreleased=frozenset({"ryo"})).run(Request("push", ""))
    assert rows(shared) == every_row("ryo") + [each for each in every_row("tastemaker") if each[1] != "release-resolution"] + every_row("asymptote")
    # Once the release tagged ryo, Tastemaker's resolution row runs again.
    assert rows(pull_request("crates/tastemaker/cli/src/main.rs", unreleased=frozenset({"tastemaker"}))) == every_row("tastemaker")


def test_the_releases_are_asked_only_when_a_checked_product_builds_on_another() -> None:
    pending = FakePending(frozenset({"ryo"}))
    UseCase(FakeDiff(("crates/ryo/cli/src/main.rs",)), FakeBuilds(), pending).run(Request("pull_request", ""))
    assert pending.asked == 0
    UseCase(FakeDiff(("crates/tastemaker/cli/src/main.rs",)), FakeBuilds(), pending).run(Request("pull_request", ""))
    assert pending.asked == 1
