Pinned source: Ruff 0.16.5, tag commit `9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`, expression dispatch in `checkers/ast/analyze/expression.rs`; target Python 3.10.

The query follows `if_expr_min_max`: a single ordering comparison, with the conditional branches structurally equal to its two operands in either order. Builtin min/max availability affects fixes only and does not gate the diagnostic. The existing expression-shape matching policy is retained; the duplicated suppression implementation is replaced with the shared fragment.

All original 12 expected arrays are preserved. The expanded 52-case cohort includes the entire tagged FURB136 fixture, every comparison operator, calls/attributes/subscripts/lambdas/comprehensions, canonical literal spellings, differing arguments, chained-comparison controls, shadowed min/max, Unicode, and suppression.

Validation: 52/52 cases passed against both the pinned reference and frozen actual benchmark worker `5808f11d6cd198a7c672c12fee134077f21e98e31f2725e7aaa4d3f5c2146837`. Every case produced both complete records, with zero unknown rule/file pairs. Local receipts and query/case hashes: `collection-policy-20261101/verification.json`. This is bounded fixture evidence, not a whole-corpus parity certificate. No prepared-manifest registration is changed here.
