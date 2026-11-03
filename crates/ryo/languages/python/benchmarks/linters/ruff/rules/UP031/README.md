# UP031 — printf-style formatting

The source draft uses existing percent-format parse/segment facts and a new additive raw-format-field flags column. Facts retain generic parsed flag presence; conversion/operand eligibility and diagnostic span policy stay in RyoQL. Normal convertible expressions cover the whole BinOp; unsupported conversion/argument cases cover the format literal. Segment order preserves early malformed/unsupported outcomes. Dict key syntax, Unicode identifiers, duplicate keys and unpacking remain explicit.

All 37 cases match pinned Ruff0.16.5 at Python3.10 and Python3.7 (74 actual reference invocations). Both full upstream fixture files are preserved, with 68 and zero findings respectively. Independent flag, conversion, mapping, operand, ordering, shadowing, suppression and exact-span controls are included. Six actual CPython3.10 controls preserve combined/repeated flags, repr behavior and escaped percent semantics. The query emits diagnostics only; no fixes are generated.

RyoQL parser/typecheck, Rust gates and candidate behavior remain unverified until the new facts are compiled and replayed. UP031 remains FAIL. Source/helper fixed-point preflight checks closure and identities only; it is not grammar/runtime evidence.

The raw flags contract normalizes repeated flag presence to unique ASCII-order characters (space, #, +, -, 0). Conflicting flags are preserved, not simplified by conversion semantics. Raw-token metadata does not decode string escapes; decoded formatting remains a separate existing relation. Flags are captured during the parser's existing pass, use borrowed canonical text, and require no new parser/cache or operator. Fields append after existing columns. The existing catalog-schema digest in core repository service includes ordered column names/kinds and is included in request-wide stored-rule identity, so old retained shapes cannot be reused as new rows.

[Reference-only artifacts](../../results/up031-source-reference-2026-11-03/README.md) retain exact source bytes, full JSON outputs, invocations and executable hashes. No reference discrepancies are claimed. These checks are not candidate proof or a full413 result.
