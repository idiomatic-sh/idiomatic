# F401 deferred-scope caller boundary

Ruff 0.16.5 invokes `unused_import` only for module and function scopes in `analyze/deferred_scopes.rs`. Class imports define attributes and do not reach that rule caller. The query now applies that boundary before calculating import use and export policy, including ordinary and asynchronous methods, nested classes, and stub classes.

All 22 original live-integration controls are preserved. The scoped actual-worker profile passes 29/29 cases with 58 complete contestant records and zero unknown/unread outcomes on worker SHA `65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b`. The final frozen reproduction is preserved with its full report arrays in `cross-profile-cases.toml`.

Candidate-only replay of that exact source under the complete frozen 310-query inventory plus queued fixes matches the sealed reference report exactly: two complete records, zero unknown/unread or skipped pairs. Evidence: `f401-final-full310-sol/{cases.jsonl,result.json}`; scoped evidence: `f401-sol-seven/cases.jsonl`. This does not assert complete 413-rule parity.
