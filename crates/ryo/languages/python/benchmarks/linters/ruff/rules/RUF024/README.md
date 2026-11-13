Pinned source: Ruff 0.16.5, tag commit `9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`, expression dispatch in `checkers/ast/analyze/expression.rs`; target Python 3.10.

The query follows `mutable_fromkeys_value` and `typing::is_mutable_expr`: builtin dict receiver, exactly two positional arguments (keywords do not exclude a match), literal mutable containers/comprehensions, or calls to the exact builtin/collections mutable-return whitelist. A constructor subscript is removed once, matching `map_subscript`; assigned mutable values and arbitrary functions are not inferred. Diagnostics cover the complete call.

The 55 full-report cases include the entire tagged RUF024 fixture, all six literal/comprehension forms, every whitelisted constructor, generic and doubly subscripted calls, unknown/immutable controls, aliases and shadowing, argument shape, Unicode, and suppression.

Validation: 55/55 cases passed against both the pinned reference and frozen actual benchmark worker `5808f11d6cd198a7c672c12fee134077f21e98e31f2725e7aaa4d3f5c2146837`. Every case produced both complete records, with zero unknown rule/file pairs. Local receipts and query/case hashes: `collection-policy-20261101/verification.json`. This is bounded fixture evidence, not a whole-corpus parity certificate. No prepared-manifest registration is changed here.
