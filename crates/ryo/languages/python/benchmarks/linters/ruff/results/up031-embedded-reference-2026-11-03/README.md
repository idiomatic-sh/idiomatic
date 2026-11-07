# Embedded percent-format controls

Pinned Ruff 0.16.5 was invoked directly on six unchanged source controls at Python 3.10 and 3.7. Eight observations match independently specified diagnostics. Escaped-quote and escaped-backslash annotations panic at `printf_string_formatting.rs:474` with `assertion failed: start.raw <= end.raw`, exit 101, on both targets. Exact source, stdout, stderr and invocation program are retained. No upstream issue was filed.

The intended positive is the same percent-format expression used by the preserved upstream quoted-annotation fixture. Complex decoded annotations use the existing WholeHost physical mapping; independently intended spans cover that annotation literal, while candidate correctness remains unverified. The shared harness currently rejects failed reference processes. These cases remain strict FAIL: neither empty output nor a crash is treated as raw parity or a completed discrepancy certificate.
