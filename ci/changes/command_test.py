"""The `changes` job's intent: its command run end to end, every port faked through
`composition.compose`."""

from __future__ import annotations

from ci.changes import command, composition
from ci.changes.use_case_test import FakeBuilds, FakeDiff, FakeManifest, FakePending


def job(*paths: str, unreleased: frozenset[str] = frozenset()) -> composition.UseCase:
    return composition.compose(
        composition.Parts(diff=FakeDiff(paths), builds=FakeBuilds(), manifest=FakeManifest(), pending=FakePending(unreleased))
    )


def test_a_pull_request_s_products_rows_and_skips_are_printed(capsys, monkeypatch) -> None:
    monkeypatch.delenv("GITHUB_OUTPUT", raising=False)
    assert command.run(["--event=pull_request", "--product="], job("README.md")) == 0
    printed, said = capsys.readouterr()
    lines = printed.splitlines()
    assert said == ""
    assert lines[0] == 'products=["tastemaker","asymptote"]'
    assert lines[1].startswith('rows=[{"product":"tastemaker","check":{"runner":"ubuntu-26.04","groups":"release-clippy",')
    assert lines[1].count('"product":"tastemaker"') == lines[1].count('"product":"asymptote"') == 9
    assert lines[2:] == ['skips=["python-release","bench"]']


def test_on_github_actions_they_are_the_step_s_outputs(tmp_path, monkeypatch) -> None:
    output = tmp_path / "output"
    output.write_text("before=1\n")
    monkeypatch.setenv("GITHUB_OUTPUT", str(output))
    assert command.run(["--event=workflow_dispatch", "--product=tastemaker"], job(unreleased=frozenset({"ryo"}))) == 0
    lines = output.read_text().splitlines()
    assert lines[0] == "before=1"
    assert lines[1] == 'products=["tastemaker"]'
    assert lines[2].startswith(
        'rows=[{"product":"tastemaker","check":{"runner":"ubuntu-26.04","groups":"release-clippy","label":"release-clippy",'
    )
    assert '"release-resolution"' not in lines[2]
    assert lines[3:] == ['skips=["python-release","bench"]']


def test_a_refused_request_exits_2_and_writes_no_output(tmp_path, capsys, monkeypatch) -> None:
    output = tmp_path / "output"
    monkeypatch.setenv("GITHUB_OUTPUT", str(output))
    assert command.run(["--event=pull_request"], job("docs/guide.md")) == 2
    assert capsys.readouterr() == (
        "",
        "changes: docs/guide.md is neither shared nor the root's alone: name it in SHARED or ROOT_ONLY of ci/changes/use_case.py\n",
    )
    assert not output.exists()
