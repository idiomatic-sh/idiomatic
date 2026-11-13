# PLE0116 — continue in finally before Python 3.8

The query requires the file's `language-target` to be Python 3.7 or earlier. It walks a `finally` body through branches, nested `try` body and `else`, `with` body, match cases, and loop `else`; it stops at nested loop bodies and function/class scopes.

The canonical Python 3.7 profile has 12 cases, including the byte-preserved Ruff 0.16.5 `continue_in_finally.py` fixture (SHA-256 `6ff6612aa535cc74d9799027de70b4c90ad6ddfd6f90e590f931ff150dca4b02`). The exact sealed-worker singleton replay passed all 24 candidate/reference records with raw parity. The same 12 sources produced no findings under Python 3.10 in direct worker requests and pinned Ruff. The all-negative Python 3.10 profile is a configuration control; the fixture verifier's positive/negative adequacy gate correctly refuses it.

Raw receipts and source identities are preserved. Integration admission and a full 413-rule replay are separate checks.
