# Leak review: claude/review-criteria (fourth pass)

Source `59d2a731ff3a1da11466c71fd27c3b6ad816aee5`, base `e75e47b059`. Everything was read with `git show 59d2a731ff:<path>`. The hooks were run from full `git archive` copies in the session scratchpad, with their own state root:
- the source sha;
- the base;
- `73649ab28b` (the third pass);
- `9b42116ff3`, where needed.

The full hook suite passes on the archive: 168 tests, OK. The reviewed worktree stayed at `59d2a731ff` with no tracked changes.

## Third-pass findings: closed
- **2a, here-string.** `guard_shell.py:94` now skips all three characters of `<<<`, and `HEREDOC` (`:62`) has its `(?<!<)` lookbehind back.
  - These spellings exit 2 here and 0 at `73649ab28b`: `read -r base <<<"main"` with `git push origin HEAD:main` on the next line, `<<<main`, `<<<'hello'` and `<<< "word"`.
  - The test is `test_a_here_string_hides_no_later_line`, along with the refused list at `test_hooks.py:60`.
- **2b, broken gate.** `acceptance_on_stop.py:203` `state_root()` reads `STATE` from the gate's source with `ast`, without running it, and `failure`/`success` (`:257`, `:278`) hold when it cannot be read.
  - A planted syntax error now holds with exit 2; at `73649ab28b` it exits 1.
  - A planted rewrite of the `STATE` line also holds, so the hook fails closed.
- **Third-pass advisories.**
  - `fish`, `source` and `.` now count as shells, and all three pipe spellings exit 2.
  - The `BEADS:` line now covers checkpoint-wide naming.

## Earlier probes re-run
Each probe ran against `e75e47b059`, `73649ab28b` and `59d2a731ff`.

**Shell guard.** 30 probes, all unchanged or fixed:
- every heredoc spelling from the second and third passes (`<<-`, nested `bash -c "$(…)"`, `<<\EOF`, `/bin/bash`, `command`/`exec`/`sudo`, `bash -`, `tee | sh`, `&&`/`;` after the operator, apostrophes, comment and `$((…))` `<<`);
- the data cases (`git commit -F - <<'EOF'`, `cat > notes.md <<'EOF'`, `git commit -m "$(cat <<'EOF' …)"`, `gh pr create --body "$(cat <<'EOF' …)"`).

**Scope gate.** 13 probes:
- another bead's scope, a rewritten report, a child bead, a missing SCOPE, and the three FOLLOWUP bypasses all refuse;
- an own scope and a correct FOLLOWUP pass.

**New probes, all refused (exit 2):**
- `<<<` next to `<<` (`cat <<<x <<EOF`, `<<<x<<EOF`, `<<EOF<<<x`, with cargo or a push after the body);
- `<<<` inside single or double quotes, followed by a later line;
- `<<<<EOF`, a leading `<<<x cat`, `x=$(cat <<<y)`, and a backslash continuation into `<<EOF`.

**`BEADS:` spoofing:**
- an unlisted bead in a checkpoint scope is refused;
- so are a child (`ryo-k.1`) or prefix (`ryo`) of a listed bead, and a DEFER checkpoint scope;
- a `BEADS:` line added after the review is refused through `scope_sha256`;
- a checkpoint scope whose review JSON is named for another stem (`scope-review-ryo-m.json` beside `scope-cp336.md`) is refused, because `review_path` derives the name from the report's own stem.

## 1 Proof where the contract lives: PASS
No contract is asserted. The hook tests prove hook behaviour on temp fixtures.

## 2 Every enforcement claim fires: FAIL
**A here-doc whose terminator never matches hides every later line from every guard, the push guard included** (`guard_shell.py:140-145`, `without_heredoc_bodies`). When a `<<WORD` operator's delimiter line never appears exactly, the body skip runs to the end of the command. Every later line is then dropped before any guard reads it.

The three spellings below exit 2 at `e75e47b059`, with `git push origin HEAD:main` or `cargo build` on the next line. They exit 0 at `9b42116ff3`, at `73649ab28b` and at this sha. The class has been there since the heredoc change (`7203127513`), and the earlier passes missed it.
- `x=$(cat <<EOF` / `hi` / `EOF)` (also with `<<'EOF'`). Bash 5.2 accepts this: it warns "delimited by end-of-file", ends the substitution at `)`, then runs the next line. An agent can easily write `EOF)` on one line by accident.
- `echo "$(printf '%s' "the <<EOF form")"` and the same shape inside `git commit -m "$(…)"`. `logical_line` does not model quotes nested in `$(…)`, so a literal `<<EOF` inside the inner quotes reads as an operator.
- `let x=1<<y`, where a shift reads as an operator.

Required change:
- In `without_heredoc_bodies`, when the delimiter line is not found before the end of the text, keep the skipped lines and read them as commands (fail closed). A real here-doc that runs to the end of the command has nothing after it to hide, so this costs nothing. It closes every misparse in the class at once, including future ones, instead of one spelling per pass.
- Add the three spellings above, each with `git push origin HEAD:main` on the next line, to the refused list at `test_hooks.py:60`.

**Not counted (old gaps, unchanged at base):**
- `bash <<<"$(cat <<EOF … )"`. A here-string handed to a shell is a known non-goal (a command passed as one string).
- `$'it\'s'` ANSI-C quoting before a later line. The tokenizer misreads `\'`, and this exits 0 at all three shas.

## 3 Boundaries hold across crates: PASS
No Rust boundary is in the diff. The reviewer clippy carve-out has not changed.

## 4 One implementation and one representation per concept: PASS
- One scope-review validator and one `review_path` serve both the gate and the scope-review stop.
- The state root is defined once (`no_subagent_builders.py:14`), and the stop hook parses that line instead of keeping a copy.

## 5 Layer direction: PASS
No crate is touched.

## 6 Test seams stay out of production: PASS
Only tests set `CLAUDE_HOOKS_STATE`.

## 13 API stability: PASS / 16 Dependency hygiene: PASS / 17 Portable and reproducible: PASS / 18 Use what everyone uses: PASS / 20 Purposeful placement: PASS
- 13: no public Rust surface changes.
- 16: stdlib only (`ast` added).
- 17: the suite passes on a clean archive in temp dirs.
- 18: no new tool.
- 20: the logic sits in the hooks that own it.

## Advisories (no verdict effect)
- **A deferred bead on a `BEADS:` line passes the gate.** `scoper.md:81` says a checkpoint-wide scope "lists its beads", which a scoper can read as every bead of the checkpoint, deferred ones included. No `scope-review.md` row checks the line. A PROCEED checkpoint scope that writes `BEADS: ryo-p ryo-q ryo-r` and also "ryo-r is deferred" admits a builder for `ryo-r` (exit 0).
  - A `BEADS:` line inside a fenced code block (an earlier scope quoted in the report) also counts.

  Suggested fix: change `scoper.md:81` to "lists the beads it proceeds with or reshapes on one `BEADS:` line, outside code blocks", and have S2 or S10 confirm that each listed bead is designed.
- **The review file name is described in two ways.** `scope-review.md:35` and the README at `.claude/hooks/README.md:5` still say `scope-review-<bead>.json`. `scope_hold` and `NEEDS` say the gate reads `scope-review-<stem>.json`. The gate fails closed, so this is a doc gap: name it `scope-review-<report stem>.json` in both places.
- **The stop hook parses the `STATE` line.** Any refactor of `STATE` that is not `os.environ.get("<const>", "<const>")` holds every builder stop, including BLOCKED ones. This fails closed and the message names the cause, so it is acceptable.

No enforcement candidates: the finding is in hook code, and the fix is the hook itself.

VERDICT: FAIL: 2
