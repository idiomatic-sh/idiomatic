"""The `tag` job's intent: its command run end to end, every port faked through
`composition.compose`."""

from __future__ import annotations

from ci.tag import command, composition
from ci.tag.use_case_test import FakeManifest, FakeTags, Journal, created, versions


def job(journal: Journal, **head: str):
    return lambda: composition.compose(composition.Parts(manifest=FakeManifest(journal, versions(**head)), tags=FakeTags(journal)))


def test_the_released_products_are_printed_and_tagged_when_pushed(capsys, monkeypatch) -> None:
    monkeypatch.delenv("GITHUB_OUTPUT", raising=False)
    journal = Journal()
    assert command.run(["--push"], job(journal, ryo="0.2.0", tastemaker="0.2.0")) == 0
    assert capsys.readouterr().out == '["ryo", "tastemaker"]\n'
    assert [call[1] for call in created(journal)] == ["idiomatic-sh-ryo-v0.2.0", "idiomatic-sh-tastemaker-v0.2.0"]


def test_on_github_actions_the_products_are_the_step_s_output(tmp_path, monkeypatch) -> None:
    output = tmp_path / "output"
    output.write_text("before=1\n")
    monkeypatch.setenv("GITHUB_OUTPUT", str(output))
    journal = Journal()
    assert command.run(["--dry-run"], job(journal, asymptote="0.1.2")) == 0
    assert output.read_text() == 'before=1\nproducts=["asymptote"]\n'
    assert created(journal) == []


def test_a_commit_that_releases_nothing_passes_with_an_empty_list(capsys, monkeypatch) -> None:
    monkeypatch.delenv("GITHUB_OUTPUT", raising=False)
    assert command.run(["--push"], job(Journal())) == 0
    assert capsys.readouterr().out == "[]\n"


def test_a_product_without_a_version_is_refused_with_status_2(capsys, monkeypatch) -> None:
    monkeypatch.setenv("GITHUB_OUTPUT", "/nonexistent/output")
    journal = Journal()
    manifest = FakeManifest(journal, {"HEAD": {"ryo": "0.2.0"}})
    assert command.run(["--push"], lambda: composition.compose(composition.Parts(manifest=manifest, tags=FakeTags(journal)))) == 2
    assert "tag: crates/tastemaker/Cargo.toml declares no workspace version at HEAD" in capsys.readouterr().err
    assert created(journal) == [], "a refused run writes no output and creates no tag"


def test_a_mode_is_required() -> None:
    try:
        command.run([], job(Journal()))
    except SystemExit as exit:
        assert exit.code == 2
    else:
        raise AssertionError("a run without a mode was accepted")
