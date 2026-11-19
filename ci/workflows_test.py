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

from ci.changes import composition as changes_composition
from ci.changes.builds.adapters.file.adapter import FileBuilds
from ci.changes.manifest.adapters.file.adapter import FileManifest
from ci.changes.use_case import CHECK_ROWS, DECIDED, OWN_JOBS, ROOT_ONLY, RUN_TIME_READS, SHARED
from ci.changes.use_case import Plan as ChangesPlan
from ci.changes.use_case import Request as ChangesRequest
from ci.groups import composition as groups_composition
from ci.groups.use_case import RELEASE_ROWS
from ci.groups.use_case import Request as GroupsRequest
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
# The one other job command: seeding ryo's bench lock on the runner that judges it, after the check of both bench groups: what
# it found unseeded or over its best recorded (`up`), and the records of measures no tool reports any more dropped (`down`)
# (benchmarks.yml).
SEED = re.compile(r'asymptote (up --root crates/[\w-]+ --reason "seed on [\w.-]+"|down --root crates/[\w-]+)')
# The check a seed stands on: the bench group alone (the timing group repeats the whole bench, and no job runs it twice).
SEED_CHECK = "asymptote check --root crates/ryo --groups bench"
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
# `ready_for_review` starts the run a draft skipped: a draft runs no CI (every job carries `READY`).
PULL_REQUEST = {"branches": ["main"], "types": ["opened", "synchronize", "reopened", "edited", "ready_for_review"]}
# The condition every job of a workflow a pull request starts carries: anything but a pull request that is a draft.
READY = "github.event_name != 'pull_request' || !github.event.pull_request.draft"
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
# against crates.io's live index, api's semver baseline is the newest release tag. Their rows keep no store.
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

# The bench job runs every linter bench (every default verified, ryo against the pinned reference over every project, ryo's
# cost, times and growth judged against the lock) in one job on one Linux runner, its one asymptote group. Its guards keep a
# failure from turning into a pass: no skip, no masked step, exactly its one check, its environment, and receipts uploaded even
# after failure.
BENCH_JOB_GROUP = "bench"
BENCH_JOB_CHECK = f"asymptote check --root crates/ryo --groups {BENCH_JOB_GROUP}"
# The bench job sets no RAYON_NUM_THREADS of its own: the bench tool's run line pins every timed measure, once.
BENCH_JOB_ENV = {"CARGO_BUILD_JOBS": "6"}
BENCH_JOB_UPLOAD = {
    "name": "bench-${{ github.run_attempt }}",
    "path": "target/benchmark/",
    "include-hidden-files": True,
    "if-no-files-found": "error",
    "retention-days": 7,
}


def bench_job_problems(job: dict) -> list[str]:
    """What lets the bench job pass without running every bench in its group."""
    found = []
    steps = job.get("steps", [])
    if job.get("runs-on") != "ubuntu-26.04" or "container" in job:
        found.append("bench must run on the one Linux runner, outside a container")
    if "strategy" in job:
        found.append("bench runs its one group; a matrix would add groups the gate does not review")
    if not isinstance(job.get("timeout-minutes"), int) or job["timeout-minutes"] <= 360:
        found.append("bench needs headroom over its tool's 360-minute timeout")
    if job.get("if") != f"({READY}) && !contains(fromJSON(needs.changes.outputs.skips), 'bench')" or "continue-on-error" in job:
        found.append("bench may not be skipped, but where the changes leave it nothing to run, or masked")
    if job.get("env") != BENCH_JOB_ENV:
        found.append("bench changed its pinned environment")
    runs = [step.get("run") for step in steps if "run" in step]
    if runs.count(BENCH_JOB_CHECK) != 1:
        found.append("bench does not run exactly its one check")
    if any("continue-on-error" in step or "shell" in step for step in steps):
        found.append("bench steps may not mask failures or override their shell")
    if any(
        "if" in step and (step.get("name"), step["if"])
        not in {
            ("Save the installed tools", f"{MAIN} && steps.cargo_bin_cache.outputs.cache-hit != 'true'"),
            ("Bench documents and per-rule receipts", "always()"),
        }
        for step in steps
    ):
        found.append("bench has an unreviewed step condition")
    upload = steps[-1] if steps else {}
    if not upload.get("uses", "").startswith(UPLOAD) or upload.get("if") != "always()" or upload.get("with") != BENCH_JOB_UPLOAD:
        found.append("bench must upload its complete receipts even after failure")
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
            found.append(f"{name} must start on {PULL_REQUEST['types']} of a pull request, an edited title and a draft turning ready among them")
    jobs = workflow["jobs"]
    if ("publish" in jobs or any(environment(job) == "release" for job in jobs.values())) and started - {"workflow_dispatch", "workflow_run"}:
        found.append(f"{name} publishes, so it starts only by hand or on a completed check on main")
    return found


def checks(job: dict) -> list[int]:
    """The positions of the job's steps that call `asymptote check`."""
    return [index for index, step in enumerate(job["steps"]) if re.search(r"(^|\s)asymptote check\b", step.get("run", ""))]


def seeds(job: dict) -> list[int]:
    """The positions of the job's steps that seed a product's lock: ryo's bench, or Tastemaker's memory records."""
    return [index for index, step in enumerate(job["steps"]) if SEED.fullmatch(step.get("run", "").strip())]


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
    return False


def problems(workflow: dict) -> list[str]:
    """What keeps the workflow from being a thin edge over `asymptote check`: a job that calls it more than once, anywhere but
    last (but an upload of what its tools left and the save of its asymptote store), or with more than the one command line; a
    job whose asymptote store is not restored before its check and saved after it; a job that does not check and runs more than
    one job command; a job neither checking, nor waiting for one that does, nor deciding what one runs (a job such a job waits
    for); a workflow no job of which checks."""
    jobs = workflow["jobs"]
    found = []
    for name, job in jobs.items():
        if name == "bench":
            found.extend(bench_job_problems(job))
        found.extend(store_problems(name, job))
        positions = checks(job)
        saved = {index for index, step in store_steps(job) if step["uses"].startswith(SAVE)}
        if len(positions) > 1:
            found.append(f"{name} calls asymptote check {len(positions)} times")
        elif positions:
            run = re.sub(r"\$\{\{.*?\}\}", "value", job["steps"][positions[0]]["run"].strip())
            following = enumerate(job["steps"][positions[0] + 1 :], start=positions[0] + 1)
            seeding = set(seeds(job))
            if any(not step.get("uses", "").startswith(UPLOAD) and index not in saved | seeding for index, step in following):
                found.append(f"{name} does not end in its asymptote check step")
            if not CHECK.fullmatch(run) and not captured_check(run):
                found.append(f"{name} runs {run!r}, not one `asymptote check [--root <dir>] [--groups <groups>]`")
        else:
            steps = [step for step in job["steps"] if "run" in step and step.get("name") != RELEASE_STEP and step != DRAFT_REFUSAL]
            runs = [re.sub(r"\$\{\{.*?\}\}", "value", step["run"].strip()) for step in steps]
            runs = [each for each in runs if each not in SETUP]
            if seeds(job) and all(SEED.fullmatch(each) for each in runs):
                runs = []
            if len(runs) > 1 or any(not COMMAND.fullmatch(each) for each in runs):
                found.append(f"{name} does not check and does not run exactly one job command: {runs}")
            for step in job["steps"]:
                if step.get("name") == RELEASE_STEP:
                    found += [f"{name} releases with {line!r}" for line in step["run"].strip().splitlines() if not RELEASE_LINE.fullmatch(line)]
            if any(set(step.get("env", {})) - HANDED for step in job["steps"]):
                found.append(f"{name} hands a step more than {sorted(HANDED)}")

    def gated(name: str, seen: frozenset[str] = frozenset()) -> bool:
        return bool(checks(jobs[name]) or seeds(jobs[name])) or any(
            gated(each, seen | {name}) for each in needs(jobs[name]) if each not in seen
        )

    # The jobs a gated job waits for: the ones that decide what it runs (which products a push releases).
    deciding = {each for name in jobs if gated(name) for each in needs(jobs[name])}
    found += [f"{name} neither checks nor waits for a job that does" for name in jobs if not gated(name) and name not in deciding]
    if not any(checks(job) or seeds(job) for job in jobs.values()):
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


@pytest.mark.parametrize("name", ["check", "bench"])
def test_each_real_check_remains_one_check(name: str) -> None:
    """The product checks and the bench each contain one propagated check."""
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
        "check.yml must start on ['opened', 'synchronize', 'reopened', 'edited', 'ready_for_review'] of a pull request, an edited title"
        " and a draft turning ready among them"
    ]
    # Planted: without `ready_for_review`, a draft that turns ready starts no run, so the run its draft skipped never happens.
    unready = {"on": {"pull_request": {"branches": ["main"], "types": ["opened", "synchronize", "reopened", "edited"]}}, "jobs": {}}
    assert trigger_problems("check.yml", unready) == trigger_problems("check.yml", unedited)
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


def draft_problems(name: str, workflow: dict) -> list[str]:
    """What lets a workflow a pull request starts check a draft, or pass one: a job but the verdict that runs while the pull
    request is a draft (each one's condition is `READY`, or `READY` and a further condition), and a verdict that a draft skips,
    which GitHub counts as passing, or that runs on a draft without first failing it (`DRAFT_REFUSAL`)."""
    if "pull_request" not in triggers(workflow):
        return []
    skipping = {READY}
    found = []
    for job_name, job in workflow["jobs"].items():
        condition = job.get("if")
        if job_name == VERDICT:
            if condition != GATE_CONDITION:
                found.append(f"{name} skips {job_name} on a draft pull request, which GitHub counts as a pass")
            elif job.get("steps", [None])[0] != DRAFT_REFUSAL:
                found.append(f"{name} runs {job_name} on a draft pull request without failing it first")
        elif condition not in skipping and not str(condition or "").startswith(f"({READY}) && "):
            found.append(f"{name} runs {job_name} on a draft pull request")
    return found


@pytest.mark.parametrize("path", WORKFLOWS, ids=lambda path: path.name)
def test_every_job_a_pull_request_starts_skips_a_draft_and_the_gate_fails_it(path: Path) -> None:
    """A draft pull request runs no check: every job of a workflow a pull request starts is skipped but `gate`, which runs and
    fails, so a draft never shows a passing gate."""
    assert draft_problems(path.name, yaml.safe_load(read(path))) == []


def test_a_job_a_pull_request_starts_that_does_not_skip_a_draft_is_refused() -> None:
    """A draft pull request runs no check: a job without `READY`, or whose condition `READY` does not bound, runs on a draft."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    assert draft_problems("check.yml", workflow) == []
    unguarded = deepcopy(workflow)
    unguarded["jobs"]["check-linux"].pop("if")
    widened = deepcopy(workflow)
    widened["jobs"]["bench"]["if"] = f"{READY} && inputs.product == 'ryo'"
    assert draft_problems("check.yml", unguarded) == ["check.yml runs check-linux on a draft pull request"]
    assert draft_problems("check.yml", widened) == ["check.yml runs bench on a draft pull request"]


def test_a_gate_a_draft_skips_or_passes_is_refused() -> None:
    """GitHub counts a skipped required check as passing, so the gate runs on a draft and fails it first: the shape #49 merged on,
    a gate that a draft skips, is refused, and so is a gate that runs on a draft without its refusal."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    assert workflow["jobs"][VERDICT]["steps"][0] == DRAFT_REFUSAL
    skipped = deepcopy(workflow)
    skipped["jobs"][VERDICT]["if"] = f"always() && ({READY})"
    skipped["jobs"][VERDICT]["steps"].pop(0)
    assert draft_problems("check.yml", skipped) == ["check.yml skips gate on a draft pull request, which GitHub counts as a pass"]
    assert (
        "gate must wait for ['changes', 'root', 'pull-request', 'check-linux', 'check', 'python-release', 'bench'], whatever their results,"
        " a draft's too"
    ) in gate_problems(skipped)
    unrefused = deepcopy(workflow)
    unrefused["jobs"][VERDICT]["steps"].pop(0)
    assert draft_problems("check.yml", unrefused) == ["check.yml runs gate on a draft pull request without failing it first"]
    # The refusal fails only a draft, so a pull request that is ready for review reaches the verdict.
    loosened = deepcopy(workflow)
    loosened["jobs"][VERDICT]["steps"][0]["if"] = "github.event_name == 'pull_request'"
    assert draft_problems("check.yml", loosened) == ["check.yml runs gate on a draft pull request without failing it first"]
    # A workflow no pull request starts has no draft to skip.
    assert draft_problems("release.yml", yaml.safe_load(read(DIRECTORY / "release.yml"))) == []


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
    # The rows are `ci/groups`' table, which its `groups` job keeps to the product's own: the check's matrix is that job's answer.
    assert workflow["jobs"]["check"]["needs"] == "groups"
    assert workflow["jobs"]["check"]["strategy"]["matrix"]["include"] == HAND_ROWS
    assert [step["run"] for step in workflow["jobs"]["groups"]["steps"] if "run" in step] == [
        "uv run ci/groups/command.py --product ${{ inputs.product }} --rows"
    ]
    assert workflow["jobs"]["groups"]["outputs"] == {"rows": "${{ steps.groups.outputs.rows }}"}
    rows = list(RELEASE_ROWS)
    assert {row["group"] for row in rows} == released | {"repository", "lint", "clippy", "api", "check", "memory", "bench"}
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
    # A hand run checks its product with the linter benches' one group, `bench`, on Linux x86, and no retired bench group.
    rows = list(RELEASE_ROWS)
    assert {"runner": "ubuntu-26.04", "group": "bench"} in rows
    assert not [row for row in rows if row["group"].startswith("bench-")]


# The groups a run checks for each product on Linux x86, in one asymptote call so that its tools share one build.
LINUX_GROUPS = ("lint", "clippy", "check", "release-tests", "release-coverage", "memory")
# The job that decides what a run checks: the products a pull request touches, every one on a push, a hand run's one.
CHANGES = "changes"
CHANGES_RUN = "uv run ci/changes/command.py --event=${{ github.event_name }} --product=${{ inputs.product }}"
CHANGES_OUTPUTS = {name: "${{ steps.changes.outputs." + name + " }}" for name in ("products", "rows", "skips")}
# What the decided jobs expand over: the Linux job over the products, the release's check over the rows.
PRODUCTS_MATRIX = "${{ fromJSON(needs.changes.outputs.products) }}"
ROWS_MATRIX = "${{ fromJSON(needs.changes.outputs.rows) }}"
GATED = [CHANGES, "root", "pull-request", *DECIDED]


def decided(name: str) -> str:
    """The condition of a job the changes decide: it skips a draft, and runs unless the `changes` job's `skips` names it."""
    return f"({READY}) && !contains(fromJSON(needs.changes.outputs.skips), '{name}')"


# Every job but the verdict skips a draft pull request (`READY`); the jobs the changes decide skip, too, when the run leaves them
# nothing to check.
CONDITIONS = {CHANGES: READY, "root": READY, "pull-request": READY} | {name: decided(name) for name in DECIDED}
# The verdict runs on every event, a draft pull request's too: GitHub counts a skipped required check as passing.
VERDICT = "gate"
GATE_CONDITION = "always()"
# Its first step fails a draft, which runs no check, with a message that says so; nothing else runs it.
DRAFT_REFUSAL = {
    "name": "A draft pull request has no verdict",
    "if": "github.event_name == 'pull_request' && github.event.pull_request.draft",
    "run": 'echo "::error title=Draft pull request::A draft pull request runs no check, so it has no verdict: mark it ready for review,'
    ' which runs the checks."\nexit 1\n',
}
# A job may be skipped only where its own condition skips it: the jobs the `changes` job's `skips` names.
ALLOWED_SKIPS = "${{ needs.changes.outputs.skips }}"


def gate_problems(workflow: dict) -> list[str]:
    """What keeps check.yml from checking every product a run touches behind one gate: a workflow `paths` filter (a required
    check would sit pending), a job beside the gated ones and the gate, a `changes` job that runs anything but its one command or
    hands on anything but its answer, a job that waits for another but the decided jobs' wait for `changes`, a condition but
    each job's own, a decided job that expands over anything but the `changes` job's answer, a merge to main that cancels the run
    seeding the stores, a gate that misses a job or lets one skip but the ones `changes` left nothing to run."""
    found = []
    on = workflow.get("on", workflow.get(True, {}))
    if any(key in (on.get("pull_request") or {}) for key in ("paths", "paths-ignore")):
        found.append("check.yml filters pull requests by path, which leaves its required checks pending")
    jobs = workflow["jobs"]
    if set(jobs) != {*GATED, "gate"}:
        found.append(f"check.yml runs {sorted(jobs)}, not {sorted([*GATED, 'gate'])}")
    changes = jobs.get(CHANGES, {})
    if [step.get("run") for step in changes.get("steps", []) if "run" in step] != [CHANGES_RUN] or any(
        step.get("uses", "").split("@")[0] not in {"actions/checkout", "actions/setup-python", "astral-sh/setup-uv"}
        for step in changes.get("steps", [])
        if "uses" in step
    ):
        found.append(f"{CHANGES} must run its one command, `{CHANGES_RUN}`, and no filter of its own")
    if changes.get("outputs") != CHANGES_OUTPUTS:
        found.append(f"{CHANGES} must hand on its answer, {sorted(CHANGES_OUTPUTS)}, and nothing else")
    found += [
        f"{name} waits for {needs(job)}" for name, job in jobs.items() if name != "gate" and needs(job) != ([CHANGES] if name in DECIDED else [])
    ]
    found += [f"{name} is skipped when {job.get('if')}" for name, job in jobs.items() if name != "gate" and job.get("if") != CONDITIONS.get(name)]
    if jobs.get("check-linux", {}).get("strategy", {}).get("matrix") != {"product": PRODUCTS_MATRIX}:
        found.append("check-linux must run each product the changes check")
    if jobs.get("check", {}).get("strategy", {}).get("matrix") != {"include": ROWS_MATRIX}:
        found.append("check must run each row the changes check")
    if workflow.get("concurrency", {}).get("cancel-in-progress") != "${{ github.event_name != 'push' }}":
        found.append("a merge to main must never cancel the run seeding the stores")
    gate = jobs.get("gate", {})
    if gate.get("if") != GATE_CONDITION or needs(gate) != GATED:
        found.append(f"gate must wait for {GATED}, whatever their results, a draft's too")
    steps = gate.get("steps", [{}])
    if len(steps) != 2 or steps[0] != DRAFT_REFUSAL or steps[1].get("with") != {"jobs": "${{ toJSON(needs) }}", "allowed-skips": ALLOWED_SKIPS}:
        found.append("gate may let only the jobs the changes left nothing to run skip")
    return found


def test_a_pull_request_or_hand_run_checks_every_product_behind_one_gate() -> None:
    """A pull request to main checks the products it touches, a push to main every product, and a hand run the one product it
    names: each one's whole check fan-out, the rows release.yml checks a hand run with but the root row, which the `root` job runs.
    The `changes` job, not a paths filter, decides which products run, and asymptote's stores what runs inside a job. One `gate`
    job fails when a job failed or was cancelled and passes when each succeeded, or was skipped where the `changes` job left it
    nothing to run; it fails a draft, which runs nothing else."""
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
    assert linux_checking == "asymptote check --root crates/${{ matrix.product }} --groups ${{ steps.groups.outputs.groups }}"
    # It asks each product the groups of `LINUX_GROUPS` its tools declare, through the one command that keeps them.
    (asking,) = [step for step in linux["steps"] if step.get("id") == "groups"]
    assert asking["run"] == "uv run ci/groups/command.py --product ${{ matrix.product }} --groups " + ",".join(LINUX_GROUPS)
    # asymptote declares no `memory` group, so its Linux job asks the five others; ryo and Tastemaker declare all six.
    for manifest in listed(ROOT, "crates/*/asymptote.toml"):
        read(manifest)
    asked = groups_composition.build()
    assert {product: asked.run(GroupsRequest(product, LINUX_GROUPS)) for product in PRODUCTS} == {
        "ryo": LINUX_GROUPS,
        "tastemaker": LINUX_GROUPS,
        "asymptote": ("lint", "clippy", "check", "release-tests", "release-coverage"),
    }
    (checking,) = [step["run"] for step in check["steps"] if "asymptote check" in step.get("run", "")]
    assert checking == "asymptote check --root crates/${{ matrix.product }} --groups ${{ steps.groups.outputs.groups }}"
    (asking,) = [step for step in check["steps"] if step.get("id") == "groups"]
    assert asking["run"] == "uv run ci/groups/command.py --product ${{ matrix.product }} --groups ${{ matrix.check.groups }}"
    # Every product declares every group of every row, so no row of the check matrix runs less than it names.
    for row in CHECK_ROWS:
        for product in PRODUCTS:
            assert asked.run(GroupsRequest(product, tuple(row["groups"].split(",")))) == tuple(row["groups"].split(",")), (product, row)
    # The same machines and groups as the release's check, split in two: the Linux job and the rows of `CHECK_ROWS`, a row's
    # groups sharing one asymptote call and so one build.
    release = list(RELEASE_ROWS)
    here = {("ubuntu-26.04", group) for group in LINUX_GROUPS} | {(row["runner"], group) for row in CHECK_ROWS for group in row["groups"].split(",")}
    # The linter benches run in a job of their own, `bench`, the release's `bench` row.
    bench = jobs["bench"]
    assert [step["run"] for step in bench["steps"] if "asymptote check" in step.get("run", "")] == [
        "asymptote check --root crates/ryo --groups bench"
    ]
    here |= {(bench["runs-on"], "bench")}
    assert here == {(row["runner"], row["group"]) for row in release if row.get("root") != "."}
    assert [row for row in release if row.get("root") == "."] == [{"runner": "ubuntu-26.04", "group": "repository", "root": "."}]
    rows = CHECK_ROWS
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
    (refusing, deciding) = jobs["gate"]["steps"]
    assert refusing == DRAFT_REFUSAL and deciding["uses"].startswith("re-actors/")


@pytest.mark.parametrize(
    "change",
    [
        pytest.param(
            lambda workflow: workflow["jobs"]["changes"].update(
                {"steps": [{"uses": "dorny/paths-filter@ceb8a2b8f2d89434be7ff52d3de7ec3738c5cc9d"}]}
            ),
            id="paths-filter-job",
        ),
        pytest.param(
            lambda workflow: workflow["jobs"]["changes"]["steps"].insert(0, {"uses": "dorny/paths-filter@ceb8a2b8f2d89434be7ff52d3de7ec3738c5cc9d"}),
            id="paths-filter-beside-the-command",
        ),
        pytest.param(
            lambda workflow: workflow["on" if "on" in workflow else True]["pull_request"].update({"paths": ["crates/ryo/**"]}),
            id="workflow-paths-filter",
        ),
        pytest.param(
            lambda workflow: workflow["on" if "on" in workflow else True]["pull_request"].update({"paths-ignore": ["**.md"]}),
            id="workflow-paths-ignore",
        ),
        pytest.param(lambda workflow: workflow["jobs"].pop("changes"), id="no-changes-job"),
        pytest.param(lambda workflow: workflow["jobs"]["gate"]["needs"].remove("changes"), id="gate-drops-changes"),
        pytest.param(lambda workflow: workflow["jobs"]["bench"].pop("needs"), id="bench-waits-for-nothing"),
        pytest.param(lambda workflow: workflow["jobs"]["root"].update({"needs": "changes"}), id="root-waits-for-changes"),
        pytest.param(lambda workflow: workflow["jobs"]["bench"].update({"if": READY}), id="bench-ignores-the-skips"),
        pytest.param(
            lambda workflow: workflow["jobs"]["bench"].update({"if": decided("check")}), id="bench-follows-another-job-s-skip"
        ),
        pytest.param(
            lambda workflow: workflow["jobs"]["check-linux"]["strategy"]["matrix"].update({"product": '["ryo","tastemaker","asymptote"]'}),
            id="linux-ignores-the-products",
        ),
        pytest.param(lambda workflow: workflow["jobs"]["changes"]["outputs"].pop("skips"), id="changes-drops-an-output"),
        pytest.param(lambda workflow: workflow["jobs"]["check"].update({"needs": "root"}), id="check-waits"),
        pytest.param(lambda workflow: workflow["jobs"]["python-release"].update({"if": "false"}), id="python-release-skipped"),
        pytest.param(lambda workflow: workflow["jobs"]["check"].pop("if"), id="the-other-machines-on-a-pull-request"),
        pytest.param(lambda workflow: workflow["jobs"]["check-linux"].update({"if": "false"}), id="check-linux-skipped"),
        pytest.param(lambda workflow: workflow.update({"concurrency": {**workflow["concurrency"], "cancel-in-progress": True}}), id="a-merge-cancels-the-seed"),
        pytest.param(
            lambda workflow: workflow["jobs"]["check"]["strategy"]["matrix"].update({"product": '["ryo"]'}), id="one-product"
        ),
        pytest.param(
            lambda workflow: workflow["jobs"]["check"]["strategy"]["matrix"].update({"include": "${{ fromJSON(needs.changes.outputs.products) }}"}),
            id="check-ignores-the-rows",
        ),
        pytest.param(lambda workflow: workflow["jobs"]["gate"]["needs"].remove("bench"), id="gate-drops-bench"),
        pytest.param(
            lambda workflow: workflow["jobs"]["gate"]["steps"][1]["with"].update({"allowed-skips": "bench"}),
            id="bench-skippable",
        ),
        pytest.param(
            lambda workflow: workflow["jobs"]["gate"]["steps"][1]["with"].update({"allowed-skips": ALLOWED_SKIPS + ",root"}),
            id="root-skippable",
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
    assert root.get("if") == READY and "needs" not in root
    assert [step["run"] for step in root["steps"] if "asymptote check" in step.get("run", "")] == [
        "asymptote check --root . --groups repository,repository-check"
    ]
    assert "lint" in LINUX_GROUPS and any(step.get("run", "").endswith("--groups " + ",".join(LINUX_GROUPS)) for step in jobs["check-linux"]["steps"])
    # A check at the root runs every member's tools of the groups it names: the root's groups are its own, so it runs none of them.
    root_groups = {tool["group"] for tool in tomllib.loads(read(ROOT / "asymptote.toml"))["tools"]}
    assert {"repository", "repository-check"} <= root_groups
    for manifest in listed(ROOT, "crates/*/asymptote.toml"):
        assert not root_groups & {tool["group"] for tool in tomllib.loads(read(manifest))["tools"]}, manifest


class Changed:
    """The `diff` port of the `changes` job, faked: the paths a pull request changes."""

    def __init__(self, paths: tuple[str, ...]) -> None:
        self.changed = paths

    def paths(self) -> tuple[str, ...]:
        return self.changed


class Unreleased:
    """The `pending` port of the `changes` job, faked: the products whose version has no tag yet (GitHub's answer)."""

    def __init__(self, products: frozenset[str]) -> None:
        self.products = products

    def unreleased(self) -> frozenset[str]:
        return self.products


def changed(*paths: str, event: str = "pull_request", product: str = "", unreleased: frozenset[str] = frozenset()) -> ChangesPlan:
    """What the `changes` job answers for a run: the paths and the releases faked, which product reads which from the products'
    Cargo.toml and asymptote.toml as the checkout holds them."""
    for manifest in [*listed(ROOT, "crates/*/Cargo.toml"), *listed(ROOT, "crates/*/asymptote.toml")]:
        read(manifest)
    parts = changes_composition.Parts(diff=Changed(paths), builds=FileBuilds(ROOT), manifest=FileManifest(ROOT), pending=Unreleased(unreleased))
    job = changes_composition.compose(parts)
    return job.run(ChangesRequest(event, product))


def started(workflow: dict, plan: ChangesPlan) -> list[str]:
    """The jobs check.yml starts on a ready pull request whose `changes` job answered `plan`, each by the name GitHub shows: a
    job's condition and matrix read as written, every one this does not know refused rather than guessed."""
    outputs = {"products": json.dumps(list(plan.products)), "rows": json.dumps(list(plan.rows)), "skips": json.dumps(list(plan.skips))}
    found = []
    for name, job in workflow["jobs"].items():
        condition = job.get("if")
        if condition in (READY, GATE_CONDITION):
            running = True
        elif name in DECIDED and condition == decided(name):
            running = name not in json.loads(outputs["skips"])
        else:
            raise AssertionError(f"{name} runs when {condition}, which this does not read")
        if not running:
            continue
        matrix = job.get("strategy", {}).get("matrix")
        if matrix is None:
            instances = [{}]
        elif matrix == {"product": PRODUCTS_MATRIX}:
            instances = [{"product": product} for product in json.loads(outputs["products"])]
        elif matrix == {"include": ROWS_MATRIX}:
            instances = json.loads(outputs["rows"])
        else:
            raise AssertionError(f"{name} expands over {matrix}, which this does not read")
        for instance in instances:
            shown = job.get("name", name)
            for key, value in (("matrix.product", instance.get("product", "")), *((f"matrix.check.{k}", v) for k, v in instance.get("check", {}).items())):
                shown = shown.replace("${{ " + key + " }}", str(value))
            found.append(shown)
    return found


def checked(started_jobs: list[str]) -> dict[str, list[str]]:
    """The started jobs grouped by product (`root` for the jobs no product's matrix expands): `check-linux` and each release row
    by the label and runner it shows, ryo's own jobs under ryo."""
    found: dict[str, list[str]] = {}
    for each in started_jobs:
        if "(" in each:
            product, _, rest = each.partition("(")[2].rstrip(")").partition(", ")
            found.setdefault(product, []).append(f"{each.partition(' ')[0]} {rest}".strip())
        else:
            found.setdefault("ryo" if each in OWN_JOBS else "root", []).append(each)
    return found


# A product's release rows as check.yml's `check` job shows them, each `<label>, <runner>`.
RELEASE_ROW_NAMES = [
    "check release-clippy, ubuntu-26.04",
    "check api, macos-26",
    "check check-tests, ubuntu-26.04-arm",
    "check check-tests, macos-26",
    "check check-tests, windows-2025",
    "check release-msrv, ubuntu-26.04",
    "check release-dylint, ubuntu-26.04",
    "check release-hack, ubuntu-26.04",
    "check release-resolution, ubuntu-26.04",
]


def test_a_change_to_the_root_s_documents_alone_runs_the_whole_tree_readers_only() -> None:
    """A pull request that edits the README and the licence runs the root's groups, the title's check, the `changes` job and the
    gate, and the products whose tests read the whole tree (Tastemaker's check of a `git archive` of HEAD, asymptote's): their
    Linux job and every release row. ryo's tests read no such file, so none of its jobs runs."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    assert checked(started(workflow, changed("README.md", "LICENSE"))) == {
        "root": ["changes", "root", "pull-request", "gate"],
        "tastemaker": ["check-linux", *RELEASE_ROW_NAMES],
        "asymptote": ["check-linux", *RELEASE_ROW_NAMES],
    }


def test_a_change_inside_ryo_alone_runs_ryo_and_every_product_that_reads_it() -> None:
    """A pull request that edits ryo alone runs ryo's Linux job, every release row of ryo's (macOS, Windows and ARM tests, dylint,
    msrv, hack, api, resolution), its Python release build and its bench; and the same rows of Tastemaker, which builds against
    ryo's crates, and of asymptote, whose tests install Tastemaker's command line."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    assert checked(started(workflow, changed("crates/ryo/cli/src/main.rs", "crates/ryo/README.md"))) == {
        "root": ["changes", "root", "pull-request", "gate"],
        "ryo": ["check-linux", *RELEASE_ROW_NAMES, "python-release", "bench"],
        "tastemaker": ["check-linux", *RELEASE_ROW_NAMES],
        "asymptote": ["check-linux", *RELEASE_ROW_NAMES],
    }


def test_a_change_inside_tastemaker_alone_runs_its_readers_and_its_resolution_waits_for_an_unreleased_ryo() -> None:
    """A pull request that edits Tastemaker alone runs every product whose checks read Tastemaker's directory: asymptote's, and
    ryo's, whose test tools name `../tastemaker` among their inputs (crates/ryo/asymptote.toml). Tastemaker builds against ryo's
    tree, so while ryo's version has no tag its resolution row waits for the release (ryo-gnv5)."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    released = started(workflow, changed("crates/tastemaker/cli/src/main.rs"))
    assert checked(released) == {
        "root": ["changes", "root", "pull-request", "gate"],
        "ryo": ["check-linux", *RELEASE_ROW_NAMES, "python-release", "bench"],
        "tastemaker": ["check-linux", *RELEASE_ROW_NAMES],
        "asymptote": ["check-linux", *RELEASE_ROW_NAMES],
    }
    waiting = started(workflow, changed("crates/tastemaker/cli/src/main.rs", unreleased=frozenset({"ryo", "tastemaker"})))
    assert [each for each in released if each not in waiting] == ["check (tastemaker, release-resolution, ubuntu-26.04)"]
    # ryo's own resolution row never waits: ryo builds on no other product's tree.
    assert "check (ryo, release-resolution, ubuntu-26.04)" in started(workflow, changed("crates/ryo/Cargo.toml", unreleased=frozenset({"ryo"})))


@pytest.mark.parametrize("path", ["crates/asymptote/cli/src/main.rs", "crates/asymptote/tastemaker.lock"])
def test_a_change_inside_asymptote_alone_runs_asymptote_and_tastemaker(path: str) -> None:
    """No product's tools name asymptote's directory, but Tastemaker's tests read the whole tree (its replay tests copy every
    tastemaker.lock, asymptote's among them): a pull request that edits asymptote alone runs asymptote's jobs and Tastemaker's,
    and none of ryo's."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    assert checked(started(workflow, changed(path))) == {
        "root": ["changes", "root", "pull-request", "gate"],
        "tastemaker": ["check-linux", *RELEASE_ROW_NAMES],
        "asymptote": ["check-linux", *RELEASE_ROW_NAMES],
    }


@pytest.mark.parametrize("path", [".github/workflows/check.yml", ".github/workflows/release.yml"])
def test_a_change_to_the_workflows_runs_everything(path: str) -> None:
    """A pull request that edits a workflow runs every job and every product's every row, exactly as a push to main does."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    every = started(workflow, changed(path))
    assert every == started(workflow, changed(event="push"))
    assert [each for each in every if each.startswith("check-linux")] == [f"check-linux ({product})" for product in PRODUCTS]
    assert [each for each in every if each.startswith("check (")] == [
        f"check ({product}, {row['label']}, {row['runner']})" for product in PRODUCTS for row in CHECK_ROWS
    ]
    assert len(every) == 4 + len(PRODUCTS) * (1 + len(CHECK_ROWS)) + 2


def test_a_hand_run_checks_its_one_product() -> None:
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    handed = started(workflow, changed(event="workflow_dispatch", product="asymptote"))
    assert [each for each in handed if "(" not in each] == ["changes", "root", "pull-request", "gate"]
    assert {each.split("(")[1].split(",")[0].rstrip(")") for each in handed if "(" in each} == {"asymptote"}


def test_every_top_level_path_is_shared_or_the_root_s_alone() -> None:
    """Every entry at the repository's top is a product directory's parent (`crates`), shared, or the root's alone, so the
    `changes` job refuses no path a pull request can touch today; and every path a product's tool names outside the products'
    directories is shared, or a product's check could miss a change it reads. The whole tree a tool names is the `changes`
    job's `RUN_TIME_READS` readers' (Tastemaker's and asymptote's tests), checked on every change outside the products'
    directories, and the `root` job's; another product's directory a tool names makes it that product's reader."""
    tracked = subprocess.run(["git", "ls-files", "-z"], cwd=ROOT, check=True, capture_output=True, text=True).stdout.split("\0")
    tops = sorted({path.split("/")[0] for path in tracked if path})
    assert [top for top in tops if top != "crates" and top not in SHARED and top not in ROOT_ONLY] == []
    assert [each for each in (*SHARED, *ROOT_ONLY) if each not in tops] == []
    assert not set(SHARED) & set(ROOT_ONLY)
    assert unshared_inputs(SHARED) == []
    # Planted: without the formatter's configuration in the list, a change to it would skip every product's `cargo-fmt`.
    assert unshared_inputs(tuple(each for each in SHARED if each != "rustfmt.toml")) == [
        "asymptote/cargo-fmt: rustfmt.toml",
        "ryo/cargo-fmt: rustfmt.toml",
        "tastemaker/cargo-fmt: rustfmt.toml",
    ]


def unshared_inputs(shared: tuple[str, ...]) -> list[str]:
    """Each input of a product's tool outside the products' directories and the whole tree whose top-level entry `shared` does
    not name, `<product>/<tool>: <path>`."""
    found = set()
    for manifest in listed(ROOT, "crates/*/asymptote.toml"):
        for tool in tomllib.loads(read(manifest))["tools"]:
            for each in tool.get("inputs", ["."]):
                relative = Path(os.path.relpath(os.path.normpath(manifest.parent / each.split("{")[0]), ROOT)).as_posix()
                if relative not in (".", "crates") and not relative.startswith("crates/") and relative.split("/")[0] not in shared:
                    found.add(f"{manifest.parent.name}/{tool['id']}: {relative}")
    return sorted(found)


def python_release_covered(workflow: dict) -> bool:
    """The Python release-build test has a real Linux runner, runs on every run that checks ryo, a pull request's too, and is
    required by the gate."""
    jobs = workflow["jobs"]
    job = jobs.get("python-release")
    if job is None:
        return False
    return (
        needs(job) == [CHANGES]
        and job.get("if") == decided("python-release")
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


def bench_required(workflow: dict) -> bool:
    """The gate waits for bench and lets it skip only where the `changes` job left it nothing to run."""
    gate = workflow["jobs"]["gate"]
    return "bench" in needs(gate) and gate["steps"][-1]["with"]["allowed-skips"] == ALLOWED_SKIPS


def test_bench_is_required_by_the_gate() -> None:
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    assert bench_required(workflow)
    dropped = deepcopy(workflow)
    dropped["jobs"]["gate"]["needs"].remove("bench")
    skippable = deepcopy(workflow)
    skippable["jobs"]["gate"]["steps"][-1]["with"]["allowed-skips"] += ", bench"
    assert not bench_required(dropped) and not bench_required(skippable)


def test_rust_build_caches_keep_each_product_and_profile_separate() -> None:
    def one_workspace(cache: dict, source: str) -> bool:
        return cache.get("workspaces", "").splitlines() == [f"{source} -> ../../target"]

    jobs = yaml.safe_load(read(DIRECTORY / "check.yml"))["jobs"]
    keys = set()
    for name in ("check-linux", "check", "python-release", "bench"):
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
    assert len(keys) == 3
    duplicate_target = deepcopy(cache)
    duplicate_target["workspaces"] += "\ncrates/asymptote -> ../../target"
    assert not one_workspace(duplicate_target, "crates/ryo")
    release = next(step for step in release_check()["steps"] if step.get("uses", "").startswith("Swatinem/rust-cache@"))["with"]
    assert "prefix-key" not in release
    assert release["cache-workspace-crates"] is True
    assert one_workspace(release, "crates/${{ inputs.product }}")
    assert release["shared-key"] == "${{ matrix.runner }}-${{ matrix.group }}-${{ inputs.product }}"


# A hand release's check matrix: the rows the `groups` job keeps for the product.
HAND_ROWS = "${{ fromJSON(needs.groups.outputs.rows) }}"


def hand_rows(workflow: dict, product: str) -> list[dict]:
    """The rows a hand release of `product` checks: the `groups` job's answer, run over the repository's manifests, where the
    matrix is that answer, or the matrix's own rows."""
    include = workflow["jobs"]["check"]["strategy"]["matrix"]["include"]
    return list(groups_composition.build().rows(product)) if include == HAND_ROWS else include


def undeclared_rows(workflow: dict) -> list[str]:
    """Each row a hand release checks that asks a product for a group none of its tools declares, which asymptote refuses."""
    for manifest in listed(ROOT, "crates/*/asymptote.toml"):
        read(manifest)
    declared = {product: groups_composition.build().manifest.groups(product) for product in PRODUCTS}
    return [
        f"a hand release of {product} asks for {row['group']} on {row['runner']}"
        for product in PRODUCTS
        for row in hand_rows(workflow, product)
        if "root" not in row and row["group"] not in declared[product]
    ]


def test_a_hand_release_asks_each_product_only_the_groups_it_declares() -> None:
    """A hand release checks its product's rows alone: asymptote declares neither memory nor bench, Tastemaker no bench. The
    static matrix release.yml had asked every product for every row, so a hand release of either failed."""
    workflow = yaml.safe_load(read(DIRECTORY / "release.yml"))
    assert undeclared_rows(workflow) == []
    assert [row["group"] for row in hand_rows(workflow, "asymptote") if row["runner"] == "ubuntu-26.04"] == [
        "repository", "lint", "clippy", "check", "release-tests", "release-clippy", "release-coverage", "release-msrv",
        "release-dylint", "release-hack", "release-resolution",
    ]
    static = deepcopy(workflow)
    static["jobs"]["check"]["strategy"]["matrix"]["include"] = [dict(row) for row in RELEASE_ROWS]
    assert undeclared_rows(static) == [
        "a hand release of tastemaker asks for bench on ubuntu-26.04",
        "a hand release of asymptote asks for memory on ubuntu-26.04",
        "a hand release of asymptote asks for bench on ubuntu-26.04",
    ]


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


@functools.cache
def publishable(workspace: Path) -> frozenset[str]:
    """The workspace members cargo would publish (`publish` not false), read from `cargo metadata`."""
    command = ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked", "--manifest-path", str(workspace / "Cargo.toml")]
    metadata = json.loads(subprocess.run(command, capture_output=True, text=True, check=True).stdout)
    members = set(metadata["workspace_members"])
    return frozenset(package["name"] for package in metadata["packages"] if package["id"] in members and package["publish"] != [])


def release_plz_problems(product: str, configuration: dict, published: frozenset[str]) -> list[str]:
    """What keeps a product's release-plz.toml from releasing every crate it publishes as one: a publishable member with no
    `[[package]]` (release-plz would leave it at its old version while the others move), a `[[package]]` that is no publishable
    member, one outside the product's `version_group`, and a crate missing from the changelog of the one package that keeps it."""
    packages = {package["name"]: package for package in configuration.get("package", [])}
    found = [f"{product}: {name} is published but has no [[package]]" for name in sorted(published - set(packages))]
    found += [f"{product}: [[package]] {name} is no publishable member" for name in sorted(set(packages) - published)]
    found += [f"{product}: {name} is not in version_group {product}" for name, package in packages.items() if package.get("version_group") != product]
    keeping = [name for name, package in packages.items() if package.get("changelog_update")]
    if len(keeping) != 1:
        return [*found, f"{product}: {len(keeping)} packages keep the changelog, not 1"]
    included = {keeping[0], *packages[keeping[0]].get("changelog_include", [])}
    found += [f"{product}: {name} is not in {keeping[0]}'s changelog_include" for name in sorted(published - included)]
    return found


@pytest.mark.parametrize("product", PRODUCTS)
def test_release_plz_releases_every_published_crate_of_the_product(product: str) -> None:
    """Every crate a product publishes moves to the product's one version in its release pull request, its commits in the
    product's changelog."""
    workspace = ROOT / "crates" / product
    assert publishable(workspace)
    assert release_plz_problems(product, tomllib.loads(read(workspace / "release-plz.toml")), publishable(workspace)) == []


def test_a_published_crate_missing_from_release_plz_is_refused() -> None:
    """ryo's release-plz.toml lacked six published crates (derived, execution, execution-domain, execution-runtime, matching,
    operators): planted again, each is named."""
    workspace = ROOT / "crates" / "ryo"
    configuration = tomllib.loads(read(workspace / "release-plz.toml"))
    dropped = {"idiomatic-sh-ryo-derived", "idiomatic-sh-ryo-operators"}
    configuration["package"] = [package for package in configuration["package"] if package["name"] not in dropped]
    for package in configuration["package"]:
        if "changelog_include" in package:
            package["changelog_include"] = [name for name in package["changelog_include"] if name not in dropped]
    assert release_plz_problems("ryo", configuration, publishable(workspace)) == [
        "ryo: idiomatic-sh-ryo-derived is published but has no [[package]]",
        "ryo: idiomatic-sh-ryo-operators is published but has no [[package]]",
        "ryo: idiomatic-sh-ryo-derived is not in idiomatic-sh-ryo's changelog_include",
        "ryo: idiomatic-sh-ryo-operators is not in idiomatic-sh-ryo's changelog_include",
    ]


def test_the_bench_job_is_an_independent_complete_gate() -> None:
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    assert bench_job_problems(workflow["jobs"]["bench"]) == []
    assert problems(workflow) == []


def _altered_bench(change) -> dict:
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    altered = deepcopy(workflow)
    change(altered["jobs"]["bench"])
    return altered


def _check_step(job: dict) -> dict:
    return next(step for step in job["steps"] if step.get("run") == BENCH_JOB_CHECK)


@pytest.mark.parametrize(
    "change",
    [
        pytest.param(lambda job: job.__setitem__("if", False), id="job-if-false"),
        pytest.param(lambda job: job.__setitem__("continue-on-error", True), id="job-continue-on-error"),
        pytest.param(lambda job: _check_step(job).__setitem__("if", False), id="step-if-false"),
        pytest.param(lambda job: _check_step(job).__setitem__("continue-on-error", True), id="step-continue-on-error"),
        pytest.param(lambda job: _check_step(job).__setitem__("run", BENCH_JOB_CHECK + " || true"), id="masked-exit"),
        pytest.param(lambda job: _check_step(job).__setitem__("run", "asymptote check --root crates/ryo --groups lint"), id="other-group"),
        pytest.param(lambda job: _check_step(job).__setitem__("shell", "bash +e {0}"), id="shell-override"),
        pytest.param(lambda job: job.__setitem__("timeout-minutes", 360), id="no-headroom"),
        pytest.param(lambda job: job["env"].update({"RAYON_NUM_THREADS": "1"}), id="lost-environment"),
        pytest.param(lambda job: job["steps"][-1].pop("if"), id="upload-skipped-on-failure"),
        pytest.param(lambda job: job["steps"][-1]["with"].__setitem__("if-no-files-found", "warn"), id="receipts-optional"),
        pytest.param(lambda job: job.__setitem__("container", "example.invalid/custom:latest"), id="container"),
    ],
)
def test_bench_refuses_skips_masks_and_dropped_receipts(change) -> None:
    assert problems(_altered_bench(change))


def test_the_bench_job_runs_its_one_group() -> None:
    """The job runs ryo's bench group, once: a matrix re-added, or the check pointed at another group, is refused."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    tools = tomllib.loads(read(ROOT / "crates/ryo/asymptote.toml"))["tools"]
    assert BENCH_JOB_GROUP in {tool["group"] for tool in tools}
    assert bench_job_problems(workflow["jobs"]["bench"]) == []
    matrix = deepcopy(workflow["jobs"]["bench"])
    matrix["strategy"] = {"fail-fast": False, "matrix": {"group": ["bench", "bench-cases"]}}
    other_group = deepcopy(workflow["jobs"]["bench"])
    _check_step(other_group)["run"] = "asymptote check --root crates/ryo --groups bench-cases"
    for changed in (matrix, other_group):
        assert bench_job_problems(changed)


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
    bench jobs, and nothing else is saved after a check."""
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
    """release-resolution (crates.io's live index) and api (the newest release tag) restore no store, so a
    stored pass never outlives the state it was judged on; a row restored anyway is refused."""
    workflow = yaml.safe_load(read(DIRECTORY / "check.yml"))
    rows = {row["label"] for row in CHECK_ROWS}
    assert set(UNSTORED_GROUPS) <= rows
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
    assert receipts == [("benchmarks.yml", "bench-seed"), ("benchmarks.yml", "memory-seed"), ("check.yml", "bench")]
    warm = yaml.safe_load(read(DIRECTORY / "check.yml"))["jobs"]["bench"]
    key = f"asymptote-${{{{ runner.os }}}}-${{{{ runner.arch }}}}-ryo-bench-{TOOLHASH}-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}"
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
# What a workspace's tests read at run time beyond the packages cargo builds: `RUN_TIME_READS`, ci/changes' one table.


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
    unread = unnamed_inputs(ryo, tools=[{**distribution, "inputs": [each for each in distribution["inputs"] if each != "../../tastemaker"]}])
    assert unread == ["test-distribution: tastemaker"]
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


BENCH = "languages/{lang}/benchmarks/linters/{linter}"
COLLECTOR = ["cargo", "run", "--locked", "--release", "-p", "idiomatic-sh-ryo-benchmark", "--bin", "ryo-benchmark-collector", "--"]
# The one pin of every timed measure: the whole process tree of the bench on processors 0-3 with four workers.
PIN = ["taskset", "-c", "0-3", "env", "RAYON_NUM_THREADS=4"]
# The deterministic measure held to its best exactly, with no tolerance: how many files the store holds.
EXACT = {"store-files"}
# The tools the one bench replaced: the verification, the corpus measure and parity, the verify times and growth, and the runs
# over ruff with Tastemaker's rules, which left ryo.
RETIRED = {
    "bench-correctness", "bench-measure", "bench-parity", "bench-verify", "bench-growth", "bench-replay", "memory-lint",
    "memory-dhat", "performance-lint",
}
UNFILTERED_TESTS = {
    "test-cli": ["cargo", "nextest", "run", "--workspace", "--locked", "--all-features", "-E", "package(=idiomatic-sh-ryo)"],
    "test-default": ["cargo", "nextest", "run", "--workspace", "--locked"],
    "coverage": ["cargo", "llvm-cov", "--workspace", "--all-features", "--locked"],
}


def bench_tool_problems(tools: list[dict], job: dict) -> list[str]:
    """What keeps each language bench from being one tool of the `bench` group: not exactly one tool in the group, a run that is
    not the pin followed by the collector's bench verb over the captured bench directory, an output other than BMF, a tolerance
    table other than the bench's, an input under Tastemaker, a retired tool back, or a bench job that sets a worker count of its
    own."""
    found = []
    benches = [tool for tool in tools if tool.get("group") == "bench"]
    if len(benches) != 1:
        return [f"the bench group holds {len(benches)} tools, not one"]
    (tool,) = benches
    if tool.get("run") != [*PIN, *COLLECTOR, "bench", BENCH] or BENCH not in tool.get("inputs", []):
        found.append("bench must run the pinned collector's bench verb over each bench directory")
    classes = measure_classes()
    if tool.get("output") != "bmf" or set(tool.get("tolerance-percent", {})) != set(classes["DETERMINISTIC"]) - EXACT:
        found.append("bench must print BMF with the bench's tolerance table")
    timing = [each for each in tools if each.get("group") == "bench-timing"]
    if [each.get("run") for each in timing] != [[*PIN, *COLLECTOR, "bench-timing", BENCH]] or any(
        set(each.get("tolerance-percent", {})) != set(classes["TIMING"]) or each.get("inputs") != tool.get("inputs") for each in timing
    ):
        found.append("bench-timing must run the pinned collector's timing verb over the bench's inputs, every time held to a tolerance")
    if any("tastemaker" in each for each in tool.get("inputs", [])):
        found.append("bench reads no Tastemaker rule")
    found += [f"{each['id']} is retired by bench" for each in tools if each.get("id") in RETIRED]
    if "RAYON_NUM_THREADS" in job.get("env", {}):
        found.append("the bench job sets a worker count beside the tool's pin")
    return found


def test_each_language_bench_is_one_pinned_tool() -> None:
    """Each linter bench is one tool of the bench group, expanded per bench directory: the pin, then the collector's bench verb,
    BMF with the bench's tolerances and no Tastemaker input; the bench job sets no worker count. Each planted violation is
    refused."""
    tools = tomllib.loads(read(ROOT / "crates/ryo/asymptote.toml"))["tools"]
    job = yaml.safe_load(read(DIRECTORY / "check.yml"))["jobs"]["bench"]
    assert bench_tool_problems(tools, job) == []

    def bench(changed: list[dict]) -> dict:
        return next(tool for tool in changed if tool.get("group") == "bench")

    unpinned = deepcopy(tools)
    bench(unpinned)["run"] = bench(unpinned)["run"][len(PIN) :]
    one_worker = deepcopy(tools)
    bench(one_worker)["run"][4] = "RAYON_NUM_THREADS=1"
    one_bench = deepcopy(tools)
    bench(one_bench)["run"][-1] = "languages/python/benchmarks/linters/ruff"
    loose = deepcopy(tools)
    del bench(loose)["tolerance-percent"]["peak-rss-bytes"]
    untimed = deepcopy(tools)
    next(tool for tool in untimed if tool.get("group") == "bench-timing")["tolerance-percent"].pop("wall-s")
    tastemaker = deepcopy(tools)
    bench(tastemaker)["inputs"].append("../../tastemaker")
    second = deepcopy(tools)
    second.append({**bench(second), "id": "bench-replay"})
    retired = deepcopy(tools)
    retired.append({"id": "memory-lint", "group": "memory", "run": ["cargo", "bench"]})
    for changed in (unpinned, one_worker, one_bench, loose, untimed, tastemaker, second, retired):
        assert bench_tool_problems(changed, job)
    assert bench_tool_problems(tools, {**job, "env": {**job["env"], "RAYON_NUM_THREADS": "1"}})


def test_the_test_tools_filter_no_test_out() -> None:
    """The test tools run every test of the suite; one that filters a test out is refused."""
    tools = {tool["id"]: tool for tool in tomllib.loads(read(ROOT / "crates/ryo/asymptote.toml"))["tools"]}
    assert {identity: tools[identity]["run"] for identity in UNFILTERED_TESTS} == UNFILTERED_TESTS


# The measures the collector classes as deterministic (`DETERMINISTIC` in the BMF evidence constants), the one definition the
# gate reads; every other measure the document writes is a time.
EVIDENCE_CONSTANTS = ROOT / "crates/ryo/crates/benchmark/src/outbound/evidence/adapters/bmf/constants.rs"


def measure_classes() -> dict[str, list[str]]:
    """Each class the collector's evidence constants name (`DETERMINISTIC`, `TIMING`) and the measures in it, by name."""
    text = read(EVIDENCE_CONSTANTS)
    names = dict(re.findall(r'pub\(super\) const ([A-Z_]+): &str = "([a-z-]+)";', text))
    classes = {}
    for name, members in re.findall(r"pub\(super\) const (DETERMINISTIC|TIMING): \[&str; \d+\] = \[([^\]]*)\];", text):
        classes[name] = [names[member.strip()] for member in members.split(",") if member.strip()]
    return classes


def timing_gate_problems(manifest: dict, workflows: dict[str, dict]) -> list[str]:
    """What lets a timing measure judge a pull request: the gating bench tool holding a timing measure to a tolerance, or
    publishing through any verb but `bench`, and a workflow a pull request starts running the timing group."""
    classes = measure_classes()
    tools = {tool["id"]: tool for tool in manifest["tools"]}
    found = []
    gating = [tool for tool in manifest["tools"] if tool["group"] == BENCH_JOB_GROUP]
    for tool in gating:
        timed = sorted(set(tool.get("tolerance-percent", {})) - set(classes["DETERMINISTIC"]))
        found += [f"{tool['id']} holds the time {measure} to a tolerance" for measure in timed]
        if tool["run"][-2] != "bench":
            found.append(f"{tool['id']} publishes through {tool['run'][-2]}, not the deterministic `bench`")
    timing = tools.get("bench-timing", {})
    if timing.get("group") == BENCH_JOB_GROUP or timing.get("run", [None, None])[-2] != "bench-timing":
        found.append("the timing measures are not bench-timing's, outside the gating group")
    for name, workflow in workflows.items():
        if "pull_request" not in triggers(workflow):
            continue
        for job_name, job in workflow["jobs"].items():
            for step in job.get("steps", []):
                matched = re.search(r"--groups (\S+)", step.get("run", ""))
                if matched and "bench-timing" in matched[1].split(","):
                    found.append(f"{name} runs bench-timing in {job_name}, which a pull request starts")
    return found


def test_a_timing_measure_never_gates_the_pull_request_bench() -> None:
    """The pull request's `bench` job judges the deterministic measures alone (the store and the peak memory); the times are
    `bench-timing`'s, which no workflow a pull request starts runs (the owner's decision of 2026-11-17)."""
    classes = measure_classes()
    assert classes["DETERMINISTIC"] == ["store-bytes", "store-files", "peak-rss-bytes", "warm-peak-rss-bytes", "edited-peak-rss-bytes"]
    assert set(classes["TIMING"]) == {"cpu-s", "wall-s", "warm-cpu-s", "edited-cpu-s", "total-ms", "max-case-ms", "compile-ms", "doubling-ratio", "larger-ms"}
    manifest = tomllib.loads(read(ROOT / "crates/ryo/asymptote.toml"))
    workflows = {path.name: yaml.safe_load(read(path)) for path in WORKFLOWS}
    assert timing_gate_problems(manifest, workflows) == []
    # Planted: #49's bench tool, which held every time to a tolerance in the gating group.
    planted = deepcopy(manifest)
    (bench,) = [tool for tool in planted["tools"] if tool["id"] == "bench"]
    bench["tolerance-percent"] |= {"cpu-s": 10, "wall-s": 15}
    assert timing_gate_problems(planted, workflows) == ["bench holds the time cpu-s to a tolerance", "bench holds the time wall-s to a tolerance"]
    # Planted: the pull request's bench job running the timing group beside its own.
    gated = deepcopy(workflows)
    for step in gated["check.yml"]["jobs"]["bench"]["steps"]:
        if step.get("run") == BENCH_JOB_CHECK:
            step["run"] = BENCH_JOB_CHECK + ",bench-timing"
    assert timing_gate_problems(manifest, gated) == ["check.yml runs bench-timing in bench, which a pull request starts"]


# The command every bench tool runs: one full bench, whatever measures it publishes.
BENCH_RUN = "ryo-benchmark-collector"


def groups_of(job: dict) -> list[set[str]]:
    """The groups each instance of the job checks: each `--groups` of its steps, a matrix row's (`matrix.check.groups`,
    `matrix.group`) or the groups a `groups` step names (`steps.groups.outputs.groups`) expanded per row."""
    rows = job.get("strategy", {}).get("matrix", {}).get("check") or job.get("strategy", {}).get("matrix", {}).get("include")
    rows = {HAND_ROWS: [dict(row) for row in RELEASE_ROWS], ROWS_MATRIX: [dict(row) for row in CHECK_ROWS]}.get(rows, rows) if isinstance(rows, str) else rows
    rows = rows if isinstance(rows, list) else [{}]
    named = {step.get("id"): step.get("run", "") for step in job.get("steps", [])}
    found = []
    for row in rows:
        listed: set[str] = set()
        for step in job.get("steps", []):
            run = step.get("run", "")
            if not run.startswith("asymptote check"):
                continue
            run = run.replace("${{ steps.groups.outputs.groups }}", (re.search(r"--groups (\S+)", named.get("groups", "")) or [None, ""])[1])
            run = run.replace("${{ matrix.check.groups }}", row.get("groups", "")).replace("${{ matrix.group }}", row.get("group", ""))
            matched = re.search(r"--groups (\S+)", run)
            listed |= set(matched[1].split(",")) if matched else set()
        found.append(listed)
    return found


def twice_problems(workflows: dict[str, dict], tools: list[dict]) -> list[str]:
    """Each job instance whose groups hold more than one tool that runs the whole bench."""
    benches = {tool["group"] for tool in tools if BENCH_RUN in tool["run"]}
    found = []
    for name, workflow in workflows.items():
        for job_name, job in workflow["jobs"].items():
            for listed in groups_of(job):
                running = sorted(listed & benches)
                if len(running) > 1 or any(sum(1 for tool in tools if tool["group"] == group and BENCH_RUN in tool["run"]) > 1 for group in running):
                    found.append(f"{name} runs the bench twice in {job_name}: {running}")
    return sorted(set(found))


def test_no_job_runs_the_bench_twice() -> None:
    """A full bench run takes hours: no job checks two groups whose tools each run it, and no group holds two such tools.
    `bench-timing` repeats the bench, so no workflow runs it."""
    workflows = {path.name: yaml.safe_load(read(path)) for path in WORKFLOWS}
    tools = tomllib.loads(read(ROOT / "crates/ryo/asymptote.toml"))["tools"]
    assert twice_problems(workflows, tools) == []
    # Planted: the cancelled seed run's job, which checked both bench groups.
    planted = deepcopy(workflows)
    for step in planted["benchmarks.yml"]["jobs"]["bench-seed"]["steps"]:
        if step.get("run") == SEED_CHECK:
            step["run"] = SEED_CHECK + ",bench-timing"
    assert twice_problems(planted, tools) == ["benchmarks.yml runs the bench twice in bench-seed: ['bench', 'bench-timing']"]
    # Planted: a second full-bench tool in the gating group.
    doubled = [*tools, {**next(tool for tool in tools if tool["id"] == "bench-timing"), "group": "bench"}]
    assert twice_problems(workflows, doubled) == [
        "benchmarks.yml runs the bench twice in bench-seed: ['bench']",
        "check.yml runs the bench twice in bench: ['bench']",
        "release.yml runs the bench twice in check: ['bench']",
    ]
    assert not [name for name, workflow in workflows.items() if any("bench-timing" in str(job) for job in workflow["jobs"].values())]


def test_the_bench_lock_is_seeded_only_by_hand() -> None:
    """benchmarks.yml, started only by hand, seeds ryo's bench lock, one linter bench a step, and uploads the lock and the
    receipts its tools wrote; check.yml seeds nothing."""
    workflow = yaml.safe_load(read(DIRECTORY / "benchmarks.yml"))
    assert triggers(workflow) == {"workflow_dispatch"}
    assert list(workflow["jobs"]) == ["bench-seed", "memory-seed"]
    # One seed a run, picked by the `seed` input: ryo's bench lock, or Tastemaker's memory records.
    assert workflow[True]["workflow_dispatch"]["inputs"]["seed"]["options"] == ["ryo-bench", "tastemaker-memory"]
    memory = workflow["jobs"]["memory-seed"]
    assert memory["if"] == "${{ inputs.seed == 'tastemaker-memory' }}"
    assert [step["run"] for step in memory["steps"] if "run" in step and step["run"] not in SETUP] == [
        "asymptote check --root crates/tastemaker --groups memory",
        'asymptote up --root crates/tastemaker --reason "seed on ubuntu-26.04"',
        "asymptote down --root crates/tastemaker",
    ]
    assert memory["steps"][-1]["with"]["path"] == "crates/tastemaker/.asymptote/operations/"
    job = workflow["jobs"]["bench-seed"]
    assert job["if"] == "${{ inputs.seed == 'ryo-bench' }}"
    assert "needs" not in job and "strategy" not in job
    runs = [step["run"] for step in job["steps"] if "run" in step and step["run"] not in SETUP]
    assert runs == [SEED_CHECK, 'asymptote up --root crates/ryo --reason "seed on ubuntu-26.04"', "asymptote down --root crates/ryo"]
    # `up` and `down` run after a check that finished, red (the lock did not hold this runner's numbers) or green, and `down`
    # only after `up` recorded.
    (up,) = [step for step in job["steps"] if step.get("id") == "up"]
    assert up["if"] == "${{ !cancelled() && (steps.check.outcome == 'success' || steps.check.outcome == 'failure') }}"
    (down,) = [step for step in job["steps"] if step.get("run") == "asymptote down --root crates/ryo"]
    assert down["if"] == "${{ !cancelled() && steps.up.outcome == 'success' }}"
    assert not any("continue-on-error" in step for step in job["steps"])
    upload = job["steps"][-1]
    assert upload["uses"].startswith(UPLOAD) and upload["if"] == "always()"
    assert upload["with"]["path"].split() == ["crates/ryo/.asymptote/operations/", "target/benchmark/"]
    assert "asymptote up" not in read(DIRECTORY / "check.yml")


def pull_request_time_problems(manifests: dict[str, dict], check: str) -> list[str]:
    """Every tool, in any product, of a group the check workflow asks for (which a pull request runs) that holds a wall time (a
    measure in `-ms` or `-s`) to a tolerance: a time on a shared runner never judges a pull request (the owner's decision of
    2026-11-17), in any product, not only ryo's bench."""
    asked = {group for listed in re.findall(r"--groups (\S+)", check) if "${{" not in listed for group in listed.split(",")}
    found = []
    for name, manifest in manifests.items():
        for tool in manifest["tools"]:
            if tool["group"] in asked:
                timed = sorted(measure for measure in tool.get("tolerance-percent", {}) if re.search(r"-m?s$", measure))
                found += [f"{name}: {tool['id']} ({tool['group']}) holds the time {measure} to a tolerance" for measure in timed]
    return found


def test_no_tool_a_pull_request_runs_holds_a_time() -> None:
    manifests = {str(path.parent.relative_to(ROOT)): tomllib.loads(read(path)) for path in [ROOT / "asymptote.toml", *listed(ROOT, "crates/*/asymptote.toml")]}
    check = read(DIRECTORY / "check.yml")
    assert pull_request_time_problems(manifests, check) == []
    # Planted: Tastemaker's memory tool as #52 first had it, holding `validate`'s wall times beside its peaks.
    planted = deepcopy(manifests)
    (memory,) = [tool for tool in planted["crates/tastemaker"]["tools"] if tool["id"] == "memory-distribution"]
    memory["tolerance-percent"] |= {"validate-cold-ms": 25, "validate-warm-ms": 50}
    assert pull_request_time_problems(planted, check) == [
        "crates/tastemaker: memory-distribution (memory) holds the time validate-cold-ms to a tolerance",
        "crates/tastemaker: memory-distribution (memory) holds the time validate-warm-ms to a tolerance",
    ]


MOLD = "rui314/setup-mold@"
LINKED = re.compile(r"(^|\s)(cargo |uv run --script tools/install\.py)")


def unlinked_problems(workflow: dict) -> list[str]:
    """A Linux job outside a container that builds Rust (cargo, or install.py building the tree's tools) sets up mold first: the
    repository's .cargo/config.toml links with it on Linux, and a runner without it fails with `cannot find 'ld'`."""
    found = []
    for name, each in (workflow.get("jobs") or {}).items():
        if each.get("container") or not str(each.get("runs-on", "")).startswith("ubuntu"):
            continue
        mold = False
        for step in each.get("steps", []):
            mold = mold or str(step.get("uses", "")).startswith(MOLD)
            if LINKED.search(str(step.get("run", ""))) and not mold:
                found.append(f"{name}: builds Rust before setting up mold")
                break
    return found


@pytest.mark.parametrize("path", WORKFLOWS, ids=lambda path: path.name)
def test_every_linux_job_that_builds_rust_sets_up_mold_first(path: Path) -> None:
    assert unlinked_problems(yaml.safe_load(read(path))) == []


def test_a_linux_job_that_installs_without_mold_is_refused() -> None:
    bare = {"jobs": {"check": {"runs-on": "ubuntu-26.04", "steps": [{"run": "uv run --script tools/install.py"}]}}}
    linked = {"jobs": {"check": {"runs-on": "ubuntu-26.04", "steps": [{"uses": MOLD + "x"}, {"run": "uv run --script tools/install.py"}]}}}
    assert unlinked_problems(bare) == ["check: builds Rust before setting up mold"]
    assert unlinked_problems(linked) == []
