"""The `perf` job's intent: its command run end to end, every port faked through
`composition.compose`."""

from __future__ import annotations

import subprocess

from ci.perf import command, composition
from ci.perf.use_case import ROUNDS
from ci.perf.use_case_test import HEAD, FakeBench, FakeReport, FakeTree, Journal


def job(journal: Journal, **bench):
    def compose(_output):
        tree = FakeTree(journal, ("crates/ryo/cli/src/main.rs",))
        return composition.compose(composition.Parts(head=HEAD, tree=tree, bench=FakeBench(journal, **bench), report=FakeReport(journal)))

    return compose


def test_a_pull_request_s_speed_is_judged_against_its_base() -> None:
    journal = Journal()
    assert command.run(["--against", "origin/main", "--bench", "commands", "--bench", "lint"], job(journal)) == 0
    assert {call[3] for call in journal.calls if call[0] == "measure"} == {"commands", "lint"}
    assert all(call[3] == "pass" for call in journal.calls if call[0] == "ratio")


def test_a_slower_change_fails_with_status_1() -> None:
    journal = Journal()
    assert command.run(["--against", "origin/main", "--bench", "lint"], job(journal, slower={"ryo/lint/check/cpu-ms": [1.3] * ROUNDS})) == 1


def test_the_weekly_record_runs_once_over_the_checkout() -> None:
    journal = Journal()
    assert command.run(["--bench", "distribution", "--output", "benchmarks"], job(journal)) == 0
    assert [call[4] for call in journal.calls if call[0] == "measure"] == ["head"] * 3


def test_no_bench_is_refused_with_status_2(capsys) -> None:
    assert command.run(["--against", "origin/main"], job(Journal())) == 2
    assert "no bench named" in capsys.readouterr().err


def test_an_unknown_bench_is_a_usage_error() -> None:
    try:
        command.run(["--bench", "clippy"], job(Journal()))
    except SystemExit as exit:
        assert exit.code == 2
    else:
        raise AssertionError("an unknown bench was accepted")


def test_an_invalid_measured_comparison_is_refused_with_status_2(capsys) -> None:
    journal = Journal()
    measurements = {"head-1": {}}
    assert command.run(["--against", "origin/main", "--bench", "lint"], job(journal, measurements=measurements)) == 2
    assert "same nonempty metric set" in capsys.readouterr().err
    assert not any(call[0] == "ratio" for call in journal.calls)


def test_failed_child_compilation_preserves_operational_status_and_both_streams(capsys) -> None:
    class FailedJob:
        def run(self, _request):
            raise subprocess.CalledProcessError(101, ["cargo", "bench", "--locked"], output="partial BMF", stderr="error[E0001]: compile failed")

    assert command.run(["--bench", "commands"], lambda _output: FailedJob()) == 101
    captured = capsys.readouterr()
    assert captured.out == ""
    assert "cargo bench --locked failed with exit 101" in captured.err
    assert "partial BMF" in captured.err
    assert "error[E0001]: compile failed" in captured.err
