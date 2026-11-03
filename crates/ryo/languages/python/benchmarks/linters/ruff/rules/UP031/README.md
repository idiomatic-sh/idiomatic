# UP031 — printf-style formatting

The query uses parsed percent-format segments and generic raw-format-field flags. Operand/conversion eligibility and diagnostic spans stay in RyoQL. Direct semantic-child ownership of visible `%` tokens includes quoted annotations without ambiguous mapped-span joins. The query emits diagnostics only. Repeated flags preserve unique ASCII-order presence; old columns and the existing catalog-schema cache identity remain intact.

All43 original sources and expected arrays are preserved, including both complete upstream fixtures (68 and zero findings), source-map controls and the Python3.7 countertarget. Final two-architecture certificates have passed ARM strict verification and the shared offline validator on both targets: **43 complete pairs =41 raw-exact +2 documented reference-process discrepancies**, with zero contract-only passes/errors. X86 strict verification is tracked separately and must complete before public promotion; its binary-specific capture groups are already archived. UP031 remains FAIL until that final proof is accepted.

## Reference process discrepancy

The exact two retained inputs are:

```python
x: "\"%s\" % value"
x: "'%s\\path' % value"
```

Pinned Ruff0.16.5 file-mode execution exits2 with structured `panic` output and `assertion failed: start.raw <= end.raw` at `printf_string_formatting.rs:474`. Ryo intends one UP031 finding per input at `case.py[3,19)` and `case.py[3,23)` respectively. CPython3.10 independently accepts the source, observes decoded percent-format expressions and those physical host spans, and preserves equivalent results for the specific literal formatting controls. This does not assert that the annotations evaluate to valid types. The diagnostic span uses the established embedded-source mapping contract.

The certificates bind actual `--isolated --select UP031 --target-version py310|py37 --cache-dir $STATE/ruff` settings and two observations for each exact reference executable: ARM `52469aac…` and x86 `024cdfbb…`. Only the declared input-path JSON slots vary with the authoritative request root; all remaining stdout/stderr bytes are exact. Signals, deadlines, spawn failures, changed bytes and incomplete/wrong candidate diagnostics still fail. No upstream issue was opened.

[Actual harness captures, native pins, target settings and independent controls](../../results/up031-process-harness-2026-11-03/README.md) retain authoritative bytes. [Earlier embedded controls](../../results/up031-embedded-reference-2026-11-03/README.md) preserve historical stdin101 panics and the original query failure; these are not relabeled as current file-mode observations. [Original reference checks](../../results/up031-source-reference-2026-11-03/README.md) remain immutable.
