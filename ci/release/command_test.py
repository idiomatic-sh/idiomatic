"""The `release` job's intent: its command run end to end, every port faked through
`composition.compose`."""

from __future__ import annotations

from ci.release import command, composition
from ci.release.use_case_test import FakeManifest, Journal, registries


def job(journal: Journal):
    return lambda _dist: composition.compose(composition.Parts(manifest=FakeManifest(), registries=registries(journal)))


def test_release_runs_end_to_end() -> None:
    """A dry run builds, smokes and dry-publishes every registry of every shipped product, and
    uploads nothing."""
    journal = Journal()
    assert command.run(["--dist", "dist", "--dry-run"], job(journal)) == 0
    steps = {(call[0], call[1], call[2]) for call in journal.calls}
    assert steps == {
        (step, registry, name)
        for step in ("build", "smoke", "publish")
        for registry in ("cargo", "npm", "pypi", "github")
        for name in ("alpha", "beta")
        if (step, registry) not in (("smoke", "cargo"), ("smoke", "github"), ("publish", "github"))
    }
    assert all(call[4] for call in journal.calls if call[0] == "publish"), "a dry run never uploads"


def test_release_publishes_only_when_asked_and_only_the_named_products() -> None:
    journal = Journal()
    assert command.run(["--dist", "dist", "--publish", "--product", "alpha"], job(journal)) == 0
    uploads = [call for call in journal.calls if call[0] == "publish" and not call[4]]
    assert {call[2] for call in uploads} == {"alpha"}
    assert [call[1] for call in uploads][0] == "cargo", "crates.io first"


def test_release_refuses_nothing_shipped_with_status_2(capsys) -> None:
    journal = Journal()
    refused = lambda _dist: composition.compose(composition.Parts(manifest=FakeManifest(()), registries=registries(journal)))
    assert command.run(["--dist", "dist", "--dry-run"], refused) == 2
    assert journal.calls == []
    assert "no product ships" in capsys.readouterr().err


def test_release_needs_the_artifacts_and_a_mode() -> None:
    for argv in (["--dry-run"], ["--dist", "dist"], ["--dist", "dist", "--dry-run", "--publish"]):
        try:
            command.run(argv, job(Journal()))
        except SystemExit as exit:
            assert exit.code == 2
        else:
            raise AssertionError(f"{argv} was accepted")
