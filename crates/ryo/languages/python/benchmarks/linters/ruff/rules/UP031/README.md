# UP031 — printf-style formatting

The query uses parsed percent-format segments and generic raw-format-field flags. Operand/conversion eligibility and diagnostic spans stay in RyoQL. Direct semantic-child ownership of visible `%` tokens includes quoted annotations without ambiguous mapped-span joins. The query emits diagnostics only. Repeated flags preserve unique ASCII-order presence; old columns and the existing catalog-schema cache identity remain intact.

All43 original sources and expected arrays are preserved, including both complete upstream fixtures (68 and zero findings), source-map controls and the Python3.7 countertarget. All43 now explicitly use `comparison = "full"`: the full diagnostic multiset is checked, with no dropped diagnostics. Fresh ARM strict execution and shared offline validation pass on both targets: **43 complete pairs =41 raw-exact +2 documented reference-process discrepancies**, with zero contract-only passes/errors. Earlier ARM/x86 receipts used the baseline scope (41 implicit/default Rule cases plus2 Full process cases); their raw arrays also have no extras, but those executions retain their original scope. The final combined x86 full-inventory run will exercise the strengthened source. Here Full denotes diagnostic scope, not an available reference execution report. Candidate execution accounting remains mandatory.

## Reference process discrepancy

The exact two retained inputs are:

```python
x: "\"%s\" % value"
x: "'%s\\path' % value"
```

Pinned Ruff0.16.5 file-mode execution exits2 with structured `panic` output and `assertion failed: start.raw <= end.raw` at `printf_string_formatting.rs:474`. Ryo intends one UP031 finding per input at `case.py[3,19)` and `case.py[3,23)` respectively. CPython3.10 independently accepts the source, observes decoded percent-format expressions and those physical host spans, and preserves equivalent results for the specific literal formatting controls. This does not assert that the annotations evaluate to valid types. The diagnostic span uses the established embedded-source mapping contract.

The certificates bind actual `--isolated --select UP031 --target-version py310|py37 --cache-dir $STATE/ruff` settings and two observations for each exact reference executable: ARM `52469aac…` and x86 `024cdfbb…`. Only the declared input-path JSON slots vary with the authoritative request root; all remaining stdout/stderr bytes are exact. Signals, deadlines, spawn failures, changed bytes and incomplete/wrong candidate diagnostics still fail. No upstream issue was opened.

Actual harness captures, native pins, target settings and independent controls retain authoritative bytes. Earlier embedded controls preserve historical stdin101 panics and the original query failure; these are not relabeled as current file-mode observations. Original reference checks remain immutable.
