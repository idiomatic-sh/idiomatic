"""The `groups` job's intent: its command run end to end, every port faked through
`composition.compose`."""

from __future__ import annotations

from ci.groups import command, composition
from ci.groups.use_case import RELEASE_ROWS
from ci.groups.use_case_test import DECLARED, FakeManifest


def job() -> composition.UseCase:
    return composition.compose(composition.Parts(manifest=FakeManifest(DECLARED)))


def test_the_groups_the_product_declares_are_printed(capsys, monkeypatch) -> None:
    monkeypatch.delenv("GITHUB_OUTPUT", raising=False)
    assert command.run(["--product", "asymptote", "--groups", "lint,check,memory"], job()) == 0
    assert capsys.readouterr() == ("lint,check\n", "")


def test_on_github_actions_the_groups_are_the_step_s_output(tmp_path, monkeypatch) -> None:
    output = tmp_path / "output"
    output.write_text("before=1\n")
    monkeypatch.setenv("GITHUB_OUTPUT", str(output))
    assert command.run(["--product", "ryo", "--groups", "lint,memory"], job()) == 0
    assert output.read_text() == "before=1\ngroups=lint,memory\n"


def test_a_refused_request_exits_2_and_names_no_group(tmp_path, capsys, monkeypatch) -> None:
    output = tmp_path / "output"
    monkeypatch.setenv("GITHUB_OUTPUT", str(output))
    assert command.run(["--product", "asymptote", "--groups", "memory"], job()) == 2
    assert capsys.readouterr() == ("", "groups: asymptote declares none of memory\n")
    assert not output.exists()


def test_on_github_actions_the_release_rows_are_the_step_s_output(tmp_path, monkeypatch) -> None:
    every = {row["group"] for row in RELEASE_ROWS if "root" not in row}
    output = tmp_path / "output"
    monkeypatch.setenv("GITHUB_OUTPUT", str(output))
    job = composition.compose(composition.Parts(manifest=FakeManifest({"ryo": every, "tastemaker": every, "asymptote": {"lint"}})))
    assert command.run(["--product", "asymptote", "--rows"], job) == 0
    assert output.read_text() == (
        'rows=[{"runner":"ubuntu-26.04","group":"repository","root":"."},{"runner":"ubuntu-26.04","group":"lint"}]\n'
    )
