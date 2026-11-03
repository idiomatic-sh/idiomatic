"""What the `release_pr` job's use case promises, over fakes of every port."""

from __future__ import annotations

import pytest

from ci.release_pr.check.interface import Check
from ci.release_pr.placement.interface import Placement
from ci.release_pr.release.interface import Opened, Release
from ci.release_pr.use_case import Refused, Request, UseCase, placed, title


class Journal:
    """Every call the fakes receive, in order."""

    def __init__(self) -> None:
        self.calls: list[tuple] = []


OPENED = Opened(number=40, url="https://github.com/idiomatic-sh/idiomatic/pull/40", branch="release-plz-ryo-1", versions=("0.1.3", "0.1.3"))
PLACED = (
    'templates = [\n  { template = "idiomatic.arch/cargo-workspace", file = "Cargo.toml", params = { version = "0.1.2" }, '
    'description = "The product\'s Cargo workspace." },\n]\n'
)


class FakeRelease(Release):
    def __init__(self, journal: Journal, opened: Opened | None) -> None:
        self.journal = journal
        self.opened = opened

    def open(self, product: str) -> Opened | None:
        self.journal.calls.append(("open", product))
        return self.opened


class FakePlacement(Placement):
    """The product's `tastemaker.toml` on each branch."""

    def __init__(self, journal: Journal, text: str = PLACED) -> None:
        self.journal = journal
        self.text = text

    def read(self, product: str, branch: str) -> str:
        self.journal.calls.append(("read", product, branch))
        return self.text

    def write(self, product: str, branch: str, text: str, message: str) -> None:
        self.journal.calls.append(("write", product, branch, text, message))
        self.text = text


class FakeCheck(Check):
    def __init__(self, journal: Journal) -> None:
        self.journal = journal

    def start(self, product: str, branch: str) -> None:
        self.journal.calls.append(("start", product, branch))


def use_case(journal: Journal, opened: Opened | None = OPENED, text: str = PLACED) -> UseCase:
    return UseCase(FakeRelease(journal, opened), FakePlacement(journal, text), FakeCheck(journal))


def test_the_title_is_a_conventional_commit_naming_the_product_and_version() -> None:
    assert title("tastemaker", "0.2.0") == "chore(tastemaker): release 0.2.0"


def test_the_placement_takes_the_new_version_and_nothing_else() -> None:
    assert placed(PLACED, "0.1.3") == PLACED.replace('version = "0.1.2"', 'version = "0.1.3"')


def test_a_placement_that_is_not_one_line_is_refused() -> None:
    with pytest.raises(Refused, match="places the Cargo workspace 0 times"):
        placed('tastemaker = "0.1.0"\n', "0.1.3")
    with pytest.raises(Refused, match="places the Cargo workspace 2 times"):
        placed(PLACED * 2, "0.1.3")


def test_the_pull_request_is_opened_its_placement_raised_and_its_check_started_in_that_order() -> None:
    journal = Journal()
    assert use_case(journal).run(Request("ryo")) == OPENED
    assert journal.calls == [
        ("open", "ryo"),
        ("read", "ryo", "release-plz-ryo-1"),
        ("write", "ryo", "release-plz-ryo-1", placed(PLACED, "0.1.3"), "chore(ryo): release 0.1.3"),
        ("start", "ryo", "release-plz-ryo-1"),
    ]


def test_a_placement_already_at_the_version_is_not_committed_again() -> None:
    """A rerun finds the pull request's branch placed: only its check starts again."""
    journal = Journal()
    use_case(journal, text=placed(PLACED, "0.1.3")).run(Request("ryo"))
    assert [call[0] for call in journal.calls] == ["open", "read", "start"]


def test_a_product_with_nothing_unreleased_opens_nothing_and_starts_no_check() -> None:
    journal = Journal()
    assert use_case(journal, opened=None).run(Request("asymptote")) is None
    assert journal.calls == [("open", "asymptote")]


def test_crates_moved_to_different_versions_are_refused_before_the_branch_is_touched() -> None:
    journal = Journal()
    split = Opened(number=40, url="u", branch="b", versions=("0.1.3", "0.2.0"))
    with pytest.raises(Refused, match=r"moves ryo's crates to \['0.1.3', '0.2.0'\], not one version"):
        use_case(journal, opened=split).run(Request("ryo"))
    assert journal.calls == [("open", "ryo")]
