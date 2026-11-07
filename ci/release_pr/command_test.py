"""The `release_pr` job's intent: its command run end to end, every port faked through
`composition.compose`."""

from __future__ import annotations

from ci.release_pr import command, composition
from ci.release_pr.release.interface import Opened
from ci.release_pr.use_case_test import OPENED, FakeCheck, FakePlacement, FakeRelease, Journal


def job(journal: Journal, opened: Opened | None = OPENED, text: str | None = None):
    placement = FakePlacement(journal) if text is None else FakePlacement(journal, text)
    return lambda: composition.compose(composition.Parts(release=FakeRelease(journal, opened), placement=placement, check=FakeCheck(journal)))


def test_the_pull_request_s_address_is_printed_and_its_check_started(capsys) -> None:
    journal = Journal()
    assert command.run(["--product", "ryo"], job(journal)) == 0
    assert capsys.readouterr().out == "https://github.com/idiomatic-sh/idiomatic/pull/40\n"
    assert journal.calls[-1] == ("start", "ryo", "release-plz-ryo-1")


def test_a_product_with_nothing_unreleased_passes_and_prints_no_address(capsys) -> None:
    assert command.run(["--product", "asymptote"], job(Journal(), opened=None)) == 0
    printed = capsys.readouterr()
    assert printed.out == "" and "release_pr: asymptote has nothing unreleased" in printed.err


def test_a_refused_release_pull_request_exits_with_status_2_and_starts_no_check(capsys) -> None:
    journal = Journal()
    assert command.run(["--product", "ryo"], job(journal, text='tastemaker = "0.1.0"\n')) == 2
    assert "release_pr: tastemaker.toml places the Cargo workspace 0 times, not once" in capsys.readouterr().err
    assert [call for call in journal.calls if call[0] == "start"] == []


def test_the_product_is_required_and_one_of_the_products() -> None:
    for argv in ([], ["--product", "other"]):
        try:
            command.run(argv, job(Journal()))
        except SystemExit as exit:
            assert exit.code == 2
        else:
            raise AssertionError(f"{argv} was accepted")
