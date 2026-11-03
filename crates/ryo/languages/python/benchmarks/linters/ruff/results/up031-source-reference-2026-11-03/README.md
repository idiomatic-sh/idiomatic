# UP031 reference-only source checks

All 37 intended case arrays match pinned Ruff0.16.5 at Python3.10 and Python3.7 (74 independent invocations). Both full upstream files remain intact (68 and zero findings). Expected coordinates derive from independently selected AST nodes and the preserved quoted-annotation source spelling. Full raw JSON diagnostics, exact files, arguments and executable hashes are archived. Six CPython3.10 controls retain combined/repeated flag and escaped-percent behavior.

The query uses a new additive generic raw-format-field flags column. Source closure/fixed-point checks pass; no RyoQL parser/typecheck, Rust test or candidate replay has yet run. These are reference checks, not correctness proof or an inventory promotion. UP031 remains FAIL until actual compiled-worker verification.
