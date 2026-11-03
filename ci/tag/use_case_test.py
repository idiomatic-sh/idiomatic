"""What the `tag` job's use case promises, over fakes of every port."""

from __future__ import annotations

from ci.tag.manifest.interface import Manifest
from ci.tag.tags.interface import Tags
import pytest

from ci.tag.use_case import Refused, Request, UseCase, tag


class Journal:
    """Every call the fakes receive, in order."""

    def __init__(self) -> None:
        self.calls: list[tuple] = []


class FakeManifest(Manifest):
    """Each product's version per revision; a product missing from a revision has none there."""

    def __init__(self, journal: Journal, versions: dict[str, dict[str, str]]) -> None:
        self.journal = journal
        self.versions = versions

    def version(self, product: str, revision: str) -> str | None:
        self.journal.calls.append(("version", product, revision))
        return self.versions.get(revision, {}).get(product)


class FakeTags(Tags):
    def __init__(self, journal: Journal, existing: set[str] | None = None) -> None:
        self.journal = journal
        self.existing = existing or set()

    def exists(self, tag: str) -> bool:
        self.journal.calls.append(("exists", tag))
        return tag in self.existing

    def create(self, tag: str, revision: str) -> None:
        self.journal.calls.append(("create", tag, revision))
        self.existing.add(tag)


UNCHANGED = {"ryo": "0.1.1", "tastemaker": "0.1.0", "asymptote": "0.1.1"}


def versions(**head: str) -> dict[str, dict[str, str]]:
    """The parent at `UNCHANGED`, the head with `head`'s versions over it."""
    return {"HEAD^": dict(UNCHANGED), "HEAD": UNCHANGED | head}


def created(journal: Journal) -> list[tuple]:
    return [call for call in journal.calls if call[0] == "create"]


def test_a_tag_is_named_for_its_product_and_version() -> None:
    assert tag("tastemaker", "0.2.0") == "idiomatic-sh-tastemaker-v0.2.0"


def test_a_commit_that_changes_no_version_releases_nothing() -> None:
    journal = Journal()
    assert UseCase(FakeManifest(journal, versions()), FakeTags(journal)).run(Request("HEAD", push=True)) == ()
    assert created(journal) == []
    assert not [call for call in journal.calls if call[0] == "exists"], "no tag is looked up for an unchanged version"


def test_each_product_whose_version_changed_is_released_and_tagged_at_the_commit() -> None:
    journal = Journal()
    use_case = UseCase(FakeManifest(journal, versions(ryo="0.2.0", asymptote="0.1.2")), FakeTags(journal))
    assert use_case.run(Request("HEAD", push=True)) == ("ryo", "asymptote")
    assert created(journal) == [("create", "idiomatic-sh-ryo-v0.2.0", "HEAD"), ("create", "idiomatic-sh-asymptote-v0.1.2", "HEAD")]


def test_a_dry_run_names_the_products_and_creates_no_tag() -> None:
    journal = Journal()
    use_case = UseCase(FakeManifest(journal, versions(tastemaker="0.2.0")), FakeTags(journal))
    assert use_case.run(Request("HEAD", push=False)) == ("tastemaker",)
    assert created(journal) == []


def test_a_version_whose_tag_exists_is_not_released_again() -> None:
    """A rerun of the release finds the tags its first run created."""
    journal = Journal()
    tags = FakeTags(journal, {"idiomatic-sh-ryo-v0.2.0"})
    assert UseCase(FakeManifest(journal, versions(ryo="0.2.0", tastemaker="0.2.0")), tags).run(Request("HEAD", push=True)) == ("tastemaker",)
    assert created(journal) == [("create", "idiomatic-sh-tastemaker-v0.2.0", "HEAD")]


def test_a_product_new_in_the_commit_is_released() -> None:
    journal = Journal()
    manifest = FakeManifest(journal, {"HEAD^": {"ryo": "0.1.1"}, "HEAD": {"ryo": "0.1.1", "tastemaker": "0.1.0", "asymptote": "0.1.0"}})
    assert UseCase(manifest, FakeTags(journal)).run(Request("HEAD", push=False)) == ("tastemaker", "asymptote")


def test_a_product_without_a_version_at_the_commit_is_refused_before_any_tag() -> None:
    journal = Journal()
    manifest = FakeManifest(journal, {"HEAD^": dict(UNCHANGED), "HEAD": {"ryo": "0.2.0", "tastemaker": "0.1.0"}})
    with pytest.raises(Refused, match="crates/asymptote/Cargo.toml declares no workspace version at HEAD"):
        UseCase(manifest, FakeTags(journal)).run(Request("HEAD", push=True))
    assert created(journal) == []
