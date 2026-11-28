<!-- a read-only reader's final report, 27 to 28 Nov 2026; task a098592e94dabf9d6; copied from the session transcript -->

Review of `/workspaces/ryo-poc-evidence/next` on `main`: 15 findings, 8 important, 2 pre-existing, 5 nits.

- **Scope:** there is no upstream, so I reviewed `git diff HEAD~1` (one record in `engine/STATE.md`, no issue) plus `git diff HEAD` (106 tracked files, mostly notes, about 20 scripts). The roughly 54,000 untracked entries were not reviewed.
- **Conventions:** no `CLAUDE.md` governs this tree, so there are no convention findings.
- **Not executed:** DuckDB does not load here and nothing was compiled. I ran only `translate.py`'s pure functions and a bash check; the SQL and C++ findings are from reading.
- **Prototype findings:** findings 4 to 8 and 14 are in the 20 November bake-off prototypes under `swarm/`. No exported plan triggers 4 to 7 today.
- **Untracked drivers:** `swarm/duckdb/engine.py`, `ir_json.py`, `run.py` and `load.py`, which the `translate.py` changes serve, are untracked.

```json
[
  {
    "file": "build/one-engine/scripts/build.sh",
    "line": 4,
    "summary": "INTEGRATOR_TREE does not decide which sources tm and vec are built from. The stage $target/one-path is linked once (link-release.py writes the linking tree's absolute paths into it) and the vec.rs test on line 7 skips re-linking, so a run for another tree, or for the default, compiles the first tree's code into the shared target. Fix: key the stage and target directory on the source tree, or re-link when the stage's recorded tree is not $source_tree.",
    "failure_scenario": "The stage on disk today points at /workspaces/idiomatic-target/ryo-int-base (tm-project-session path in core/Cargo.toml, the include! lines of sdk/src/linked_sdk.rs). build.sh with no INTEGRATOR_TREE (ryo-redo-ts) or with INTEGRATOR_TREE=/workspaces/ryo-int builds ryo-int-base's candidate. image_sources.rs only compares the four images with each other, so it passes and the gate measures the wrong tree.",
    "severity": "important"
  },
  {
    "file": "build/ports/stage2/dead-s2cli.sh",
    "line": 5,
    "summary": "dead-s2cli.sh no longer acts on ryo-s2cli: it now deletes in /workspaces/ryo-s2vis and writes dead-vis-$W.log and dead-vis-$W-$r.txt, the same files the tracked dead-vis.sh writes. Its name and header still say s2cli, and it still calls the old deleter del-dead-root.py, not rdel.py. Fix: restore the committed dead-s2cli.sh; dead-vis.sh already covers ryo-s2vis.",
    "failure_scenario": "`dead-s2cli.sh ryo 3`, meant for the s2cli tree, truncates the recorded dead-vis-ryo.log (line 8), overwrites dead-vis-ryo-1..3.txt and removes code from ryo-s2vis.",
    "severity": "important"
  },
  {
    "file": "build/ports/stage2/del-dead-root.py",
    "line": 32,
    "summary": "A dead-variant report makes this deleter remove the whole enum. Lines 31 to 34 walk up from the reported line to the nearest item keyword, and line 4 filters only `field` reports, so `variants ... are never constructed` lands on the `pub enum` line. The retargeted dead-s2cli.sh now feeds it ryo-s2vis reports that hold such lines. Fix: skip `variant` and `variants` messages, as rdel.py does (listed and left).",
    "failure_scenario": "dead-vis-ryo-*.txt holds `crates/core/src/outbound/lint_store/errors.rs:8:5: error: variants Unreadable and Unwritable are never constructed`. The walk-up from line 8 stops at line 5, `pub enum LintStoreError {` (checked against the file), so the lint_store port's whole error type is deleted and the crate stops compiling; the loop does not repair compile errors.",
    "severity": "important"
  },
  {
    "file": "swarm/duckdb/translate.py",
    "line": 131,
    "summary": "`in`, `not_in` and list-valued `glob_any` comparisons can never be translated. comparison() calls operand(r) on line 131 before it looks at the operand kind, and operand() raises Unsupported for `texts` and `numbers`, so lines 142-143 and 163-165 are unreachable. Fix: handle `texts` and `numbers` before calling operand(r), or build `b` only for scalar right operands.",
    "failure_scenario": "comparison(('in', ('col', 2), ('texts', ['a', 'b']))) raises Unsupported('operand:texts') (run here), so any plan whose parameter binds a list is reported as blocked. None of the three exported plans has such an operand today.",
    "severity": "important"
  },
  {
    "file": "swarm/souffle/functors.cpp",
    "line": 319,
    "summary": "A glob with `[` and no closing `]` hangs the Souffle run until memory runs out. On line 319 find() returns npos, `i = j` sets i to npos, and the loop's `++i` wraps to 0, so the glob is read again forever while `r` grows. The same branch copies `[!ab]` unchanged, which PCRE reads as a class holding `!`, not a negation. Fix: on npos emit `[` as a literal; turn a leading `!` into `^`.",
    "failure_scenario": "glob_text(\"a.py\", \"src/[ab\") never returns. glob_text(\"c.py\", \"[!ab].py\") returns 0 where globset matches. A glob with a lone `{` reaches pattern()'s abort().",
    "severity": "important"
  },
  {
    "file": "swarm/duckdb/translate.py",
    "line": 98,
    "summary": "The DuckDB glob translation disagrees with globset and with the Souffle one. glob_regex() escapes `[` and `]`, so a character class matches only its literal text; an unmatched `}` emits a bare `)`; and the test on line 115 counts every `)` in the output, including an escaped literal one. Fix: keep a brace depth counter as functors.cpp does, emit `)` only at depth above 0, and copy `[...]` classes through.",
    "failure_scenario": "Run here: 'test_[ab]*.py' gives a regex that matches only the literal text test_[ab]...; 'a}b' gives '^a)b$', an unbalanced regex that the engine should reject when the statement runs; 'x){a,b}' gives '(?:a,b)' with a literal comma, so 'x)a' does not match.",
    "severity": "important"
  },
  {
    "file": "swarm/duckdb/translate.py",
    "line": 72,
    "summary": "regex() mis-converts some Rust patterns for RE2. Class tracking is one flag, so after a nested POSIX class the next property is bracketed again; the re.sub calls on lines 90-91 rewrite a \\u escape even when its backslash is itself escaped; and the negated \\P{...} forms are not in _REGEX_FIXES. Fix: track class depth, convert \\u inside the scanning loop where escapes are already skipped, and add the \\P forms.",
    "failure_scenario": "Run here: '[[:alpha:]\\p{Number}]' becomes '[[:alpha:][\\pN]]', a class that also holds '[' followed by a literal ']'. '\\\\u0041' (a literal backslash, then u0041) becomes '\\\\x{0041}', a backslash then a repeated x. No pattern in the three exported plans has these shapes.",
    "severity": "important"
  },
  {
    "file": "swarm/souffle/functors.cpp",
    "line": 383,
    "summary": "lower() and natural_lt() call setlocale(LC_CTYPE, \"C.UTF-8\") on every call (lines 383 and 437) and ignore the result. setlocale changes the whole process and is not safe against threads that are inside towlower or iswspace, although the file header says the functors are thread-safe. Fix: make one locale with newlocale() once and use towlower_l and iswspace_l, failing loudly when it cannot be made.",
    "failure_scenario": "On an image without the C.UTF-8 locale setlocale returns NULL and towlower lowers ASCII only: lower(\"É\") returns \"É\", so a rule comparing lower(x) with a lowercase literal silently loses rows. Under souffle -j N the first calls race and every later call serialises on the libc locale lock.",
    "severity": "important"
  },
  {
    "file": "host3/scripts/warm-check",
    "line": 21,
    "summary": "warm-check always prints findings_same=0, whatever the comparison found. In run() the echo expands `$(cat $D/$1.exit)` before `$?`, so `$?` is cat's status, not compare's. The diff rewrites this line and adds a third run without fixing it. Fix: save compare's status in a variable right after the compare command and print that.",
    "failure_scenario": "A warm run whose findings differ from the oracle (compare exits 1) still prints `warm exit 0 findings_same=0 ...`. Reproduced here: `bash -c 'false; echo \"$(echo x) $?\"'` prints 0.",
    "severity": "pre_existing"
  },
  {
    "file": "build/tm-int/scripts/full.py",
    "line": 38,
    "summary": "full.py never compares the ryo workspace's answers with the golden file. The comparison on lines 38-39 runs only for executable sha 8cf15035..., whose image is no longer under tm-int/images, while the other three workspaces are compared on every run; the diff renames the file to 9a0c99610 and keeps the gate. Fix: compare ryo against the current reference for every sha, as line 37 does, or delete the pinned branch.",
    "failure_scenario": "TM_INT_WORKSPACE=ryo with a build whose answers have 46,831 rows but other bytes passes: only the row count and the per-sha baseline, written by that same sha's first run, are checked.",
    "severity": "pre_existing"
  },
  {
    "file": "build/ports/stage2/del-dead-root.py",
    "line": 8,
    "summary": "The new fixture skip also skips product modules, and the loop that calls it can no longer end early. `endswith(('cases.rs','fixtures.rs'))` matches use_cases.rs and benchmark/src/inbound/use_cases/verify_cases.rs, and dead-s2cli.sh line 12 still counts the skipped lines in `d`. rdel.py line 47 has the same suffix test. Fix: compare the basename with 'cases.rs' and 'fixtures.rs', and count only the lines the deleter acts on.",
    "failure_scenario": "With one dead helper under src/fixtures/, `dead-s2cli.sh ryo 16` runs 16 full cargo check rounds that delete nothing after the first. A dead function in use_cases/select_cases.rs is silently left.",
    "severity": "nit"
  },
  {
    "file": "build/tm-int/scripts/deletion-release-proofs.py",
    "line": 32,
    "summary": "The golden reference id is edited by hand in two scripts, although check-session/current-reference.txt is the one line the integrator changes at a switch (full-orders.sh line 11 says so). Lines 32-34 here and full.py lines 37 and 39 hard-code 9a0c99610, and only the ryo file is pinned by digest. Fix: read current-reference.txt as switch-reference.py does, and keep a digest for each of the four files.",
    "failure_scenario": "At the next reference switch these two scripts keep comparing against 9a0c99610 until someone edits them again; check-session already holds three reference sets written between 7a3ad0a6a and 9a0c99610 that they never named. A root, asymptote or tastemaker golden rewritten with the same row count is accepted.",
    "severity": "nit"
  },
  {
    "file": "engine/run-worker",
    "line": 29,
    "summary": "With RYO_ENGINE_OVERRUNS=fail as the default, a caller that ignores the worker's exit status can take a failed run for a whole one. host3/scripts/prof, changed in the same diff, records the profile and prints `done` whatever the worker returned. Fix: have prof check the worker's exit, or set RYO_ENGINE_OVERRUNS=warn for profiling runs.",
    "failure_scenario": "`prof w10 999 ./w full` on a worker where one rule overruns in one Django file: the run exits non-zero, prof still resolves the stacks and reports done, and the profile reads as a full run.",
    "severity": "nit"
  },
  {
    "file": "swarm/duckdb/check_debug.py",
    "line": 14,
    "summary": "check_debug.py still overrides translate.NO_SPAN with u64::MAX, while the diff sets the engine's value (start u32::MAX, end 0) in translate.py. Line 1 also still says the script binds views, and it now creates tables. Fix: delete line 14 and update the docstring.",
    "failure_scenario": "The SQL this script binds carries a 'no span' constant that is not the engine's. No result changes today: it binds with LIMIT 0, and the start <= end test drops a no-span value anyway.",
    "severity": "nit"
  },
  {
    "file": "build/ports/build-two.sh",
    "line": 3,
    "summary": "The label htmlbase now names another revision. build-tm.sh writes images/<label>/tm, so this run replaces the binary built from 5013b9aa with one built from a63be9dac under the same name. Fix: give the new base its own label so earlier figures still name the binary they were measured on.",
    "failure_scenario": "A note or result that cites images/htmlbase/tm from the earlier pair now points at a63be9dac's build; only images/htmlbase/revision tells the two apart.",
    "severity": "nit"
  }
]
```
